use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Generics, LitStr, Type, TypePath};

/// Derives the CLIXML serialize side of a PSRP object struct (RFC #12, L3).
///
/// Emits `From<T> for ComplexObject` and a `ToPsValue` bridge (so the type can
/// nest inside another derived struct). With `#[ps(message_type = ..)]` it also
/// emits `PsObjectWithType`, marking it a top-level message. Each field becomes
/// an `<MS>` (extended) property whose name defaults to the field name and
/// whose value is produced via `ToPsValue`; `Option<T>` fields are omitted when
/// `None`.
///
/// # Attributes
/// - `#[ps(message_type = Variant)]` (struct, optional): the
///   `MessageType::Variant` for a top-level message. Omit for sub-objects.
/// - `#[ps(name = "PropName")]` (field): override the CLIXML property name.
/// - `#[ps(adapted)]` (field): place the property in the adapted (`<Props>`) bag.
/// - `#[ps(with = "module")]` (field): use `module::to_ps_value`/`from_ps_value`
///   instead of the `ToPsValue`/`FromPsValue` traits (for primitives like
///   `Version`, or byte-backed blobs).
#[proc_macro_derive(PsSerialize, attributes(ps))]
pub fn derive_ps_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_ps_serialize(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derives `TryFrom<ComplexObject> for T` for a PSRP message struct (RFC #12, L3).
///
/// Required fields are read with `ComplexObject::req`, `Option<T>` fields with
/// `ComplexObject::opt`. Honors the same `#[ps(name = ..)]` field attribute as
/// [`macro@PsSerialize`].
#[proc_macro_derive(PsDeserialize, attributes(ps))]
pub fn derive_ps_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_ps_deserialize(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Per-field options parsed from `#[ps(..)]`.
#[allow(clippy::struct_excessive_bools)] // independent attribute flags
struct PsFieldOpts {
    ident: Ident,
    /// CLIXML property name (defaults to the field name).
    name: String,
    /// Whether the field type is `Option<..>`.
    is_option: bool,
    /// Place in the adapted (`<Props>`) bag instead of extended (`<MS>`).
    adapted: bool,
    /// On deserialize, fall back to `Default::default()` when absent (instead of
    /// erroring). For tolerant host params. Ignored for `Option<..>`.
    default: bool,
    /// For an `Option<..>` field: always emit the property (as `Nil` when
    /// `None`) instead of omitting it. Some objects require the slot present.
    nil_when_none: bool,
    /// Also set the object's `<ToString>` from this (String) field's value, in
    /// addition to emitting it as a normal property.
    set_to_string: bool,
    /// Extra property names to ALSO emit on serialize (and accept on
    /// deserialize) — e.g. a PascalCase alias alongside the camelCase name, for
    /// .NET host objects that are read under either casing.
    also: Vec<String>,
    /// Dictionary mode only: a `BTreeMap<String, PsValue>` whose entries are
    /// merged directly into the parent `<DCT>` (and, on deserialize, collect all
    /// keys not claimed by a named field).
    flatten: bool,
    /// `value_dictionary` mode only: the integer key this field occupies in the
    /// `<DCT>`.
    key: Option<i32>,
    /// `value_dictionary` mode only: the .NET type name stamped on the field's
    /// `{T, V}` value-wrapper (`T`).
    type_tag: Option<String>,
    /// Property-bag mode only: nest this field's object one extra level, under a
    /// single property of the given name (e.g. `_hostDefaultData` → `{ data: .. }`).
    wrap: Option<String>,
    /// Property-bag mode only: flatten an `Option<NestedStruct>` field into the
    /// parent, prepending this prefix to each of the nested object's property
    /// names (e.g. `ErrorCategory_` → `ErrorCategory_Reason`). On deserialize the
    /// nested struct is reconstructed from the prefixed properties, or `None`
    /// when none are present.
    flatten_prefix: Option<String>,
    /// Deserialize only: if the field's name(s) are not found at the top level,
    /// also search inside this sibling object property for the same name(s). For
    /// .NET records that nest their real payload inside an `Exception` object.
    fallback_object: Option<String>,
    /// Optional custom converter module. When set, the field is (de)serialized
    /// via `<path>::to_ps_value(&T) -> PsValue` and
    /// `<path>::from_ps_value(&PsValue) -> Result<T, PowerShellRemotingError>`
    /// instead of the `ToPsValue`/`FromPsValue` traits.
    with: Option<syn::Path>,
}

fn ps_named_fields(input: &DeriveInput) -> syn::Result<Vec<PsFieldOpts>> {
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Ps(De)Serialize requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "Ps(De)Serialize can only be derived for structs",
            ));
        }
    };

    fields
        .iter()
        .map(|field| {
            let ident = field.ident.clone().expect("named field");
            let mut name = ident.to_string();
            let mut adapted = false;
            let mut default = false;
            let mut nil_when_none = false;
            let mut set_to_string = false;
            let mut flatten = false;
            let mut also = Vec::new();
            let mut with = None;
            let mut key = None;
            let mut type_tag = None;
            let mut wrap = None;
            let mut flatten_prefix = None;
            let mut fallback_object = None;

            for attr in &field.attrs {
                if !attr.path().is_ident("ps") {
                    continue;
                }
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let lit: LitStr = meta.value()?.parse()?;
                        name = lit.value();
                    } else if meta.path.is_ident("also") {
                        let lit: LitStr = meta.value()?.parse()?;
                        also.push(lit.value());
                    } else if meta.path.is_ident("adapted") {
                        adapted = true;
                    } else if meta.path.is_ident("default") {
                        default = true;
                    } else if meta.path.is_ident("nil_when_none") {
                        nil_when_none = true;
                    } else if meta.path.is_ident("to_string") {
                        set_to_string = true;
                    } else if meta.path.is_ident("flatten") {
                        flatten = true;
                    } else if meta.path.is_ident("key") {
                        let lit: syn::LitInt = meta.value()?.parse()?;
                        key = Some(lit.base10_parse::<i32>()?);
                    } else if meta.path.is_ident("type_tag") {
                        let lit: LitStr = meta.value()?.parse()?;
                        type_tag = Some(lit.value());
                    } else if meta.path.is_ident("wrap") {
                        let lit: LitStr = meta.value()?.parse()?;
                        wrap = Some(lit.value());
                    } else if meta.path.is_ident("flatten_prefix") {
                        let lit: LitStr = meta.value()?.parse()?;
                        flatten_prefix = Some(lit.value());
                    } else if meta.path.is_ident("fallback_object") {
                        let lit: LitStr = meta.value()?.parse()?;
                        fallback_object = Some(lit.value());
                    } else if meta.path.is_ident("with") {
                        let lit: LitStr = meta.value()?.parse()?;
                        with = Some(lit.parse()?);
                    } else {
                        return Err(meta.error("unknown #[ps(..)] field attribute"));
                    }
                    Ok(())
                })?;
            }

            Ok(PsFieldOpts {
                is_option: is_option_type(&field.ty),
                ident,
                name,
                adapted,
                default,
                nil_when_none,
                set_to_string,
                also,
                flatten,
                key,
                type_tag,
                wrap,
                flatten_prefix,
                fallback_object,
                with,
            })
        })
        .collect()
}

/// Struct-level `#[ps(..)]` options.
#[derive(Default)]
struct PsStructOpts {
    /// `MessageType` variant; present only for top-level PSRP messages.
    message_type: Option<Ident>,
    /// `<TN>` type-name chain, most specific first (for typed .NET objects).
    type_names: Vec<String>,
    /// Serialize as a `<DCT>` dictionary body (string-keyed by field name)
    /// instead of an `<MS>` property bag — for PSPrimitiveDictionary-shaped
    /// objects. `Option<..>` fields are omitted when `None`.
    dictionary: bool,
    /// Serialize as a `<DCT>` keyed by each field's integer `#[ps(key = N)]`,
    /// where every value is wrapped in a `{T, V}` object (`T` = the field's
    /// `#[ps(type_tag = "..")]`, `V` = the field value). This is the shape the
    /// PowerShell host uses for its RawUI default data.
    value_dictionary: bool,
}

fn ps_struct_opts(input: &DeriveInput) -> syn::Result<PsStructOpts> {
    let mut opts = PsStructOpts::default();
    for attr in &input.attrs {
        if !attr.path().is_ident("ps") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("message_type") {
                opts.message_type = Some(meta.value()?.parse::<Ident>()?);
                Ok(())
            } else if meta.path.is_ident("type_names") {
                let content;
                syn::parenthesized!(content in meta.input);
                let names = content
                    .parse_terminated(<LitStr as syn::parse::Parse>::parse, syn::Token![,])?;
                for l in names {
                    opts.type_names.push(l.value());
                }
                Ok(())
            } else if meta.path.is_ident("dictionary") {
                opts.dictionary = true;
                Ok(())
            } else if meta.path.is_ident("value_dictionary") {
                opts.value_dictionary = true;
                Ok(())
            } else {
                Err(meta.error("unknown #[ps(..)] struct attribute"))
            }
        })?;
    }
    Ok(opts)
}

fn impl_ps_serialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let opts = ps_struct_opts(input)?;
    let fields = ps_named_fields(input)?;

    let inserts: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let bag = if f.adapted {
                quote! { adapted }
            } else {
                quote! { extended }
            };
            // `wrap`: nest the field's object one level under a single property.
            if let Some(wrapname) = &f.wrap {
                let prop = &f.name;
                return quote! {
                    obj = obj.#bag(#prop,
                        ironposh_psrp::ps_value::ComplexObject::standard()
                            .extended(
                                #wrapname,
                                ironposh_psrp::ps_value::ToPsValue::to_ps_value(&value.#ident),
                            )
                            .build_value()
                    );
                };
            }
            // `flatten_prefix`: merge a nested `Option<Struct>`'s properties into
            // the parent, each name prefixed.
            if let Some(prefix) = &f.flatten_prefix {
                return quote! {
                    if let ::core::option::Option::Some(__nested) = &value.#ident {
                        let __sub = ironposh_psrp::ps_value::ComplexObject::from(__nested);
                        for (__n, __p) in __sub.properties.iter() {
                            let __name = ::std::format!("{}{}", #prefix, __n);
                            match __p.kind {
                                ironposh_psrp::ps_value::PropertyKind::Adapted =>
                                    obj = obj.adapted(__name, __p.value.clone()),
                                ironposh_psrp::ps_value::PropertyKind::Extended =>
                                    obj = obj.extended(__name, __p.value.clone()),
                            }
                        }
                    }
                };
            }
            // Emit under the primary name plus any `also` aliases.
            let names = std::iter::once(f.name.clone()).chain(f.also.iter().cloned());
            let stmts: Vec<TokenStream2> = names
                .map(|prop| match (&f.with, f.is_option) {
                    (Some(with), true) if f.nil_when_none => quote! {
                        obj = obj.#bag(#prop, value.#ident.as_ref().map(#with::to_ps_value));
                    },
                    (Some(with), true) => quote! {
                        if let ::core::option::Option::Some(inner) = &value.#ident {
                            obj = obj.#bag(#prop, #with::to_ps_value(inner));
                        }
                    },
                    (Some(with), false) => quote! {
                        obj = obj.#bag(#prop, #with::to_ps_value(&value.#ident));
                    },
                    // Option emitted always as Nil-or-value (slot must be present).
                    (None, true) if f.nil_when_none => {
                        quote! { obj = obj.#bag(#prop, &value.#ident); }
                    }
                    (None, true) if !f.adapted => {
                        quote! { obj = obj.extended_opt(#prop, value.#ident.as_ref()); }
                    }
                    (None, _) => quote! { obj = obj.#bag(#prop, &value.#ident); },
                })
                .collect();
            let to_string_stmt = if f.set_to_string {
                // Works for `String` and any `Display` field (e.g. a polymorphic
                // union whose `<ToString>` is computed from its active variant).
                quote! { obj = obj.to_string_repr(::std::string::ToString::to_string(&value.#ident)); }
            } else {
                quote! {}
            };
            quote! { #(#stmts)* #to_string_stmt }
        })
        .collect();

    // Optional <TN> type-name chain for typed .NET objects.
    let type_names_setup = if opts.type_names.is_empty() {
        quote! {}
    } else {
        let tns = &opts.type_names;
        quote! { obj = obj.type_names([ #( ::std::borrow::Cow::Borrowed(#tns) ),* ]); }
    };

    // `PsObjectWithType` is only generated for top-level messages (those with a
    // message_type); sub-objects skip it but still get the conversions below.
    let message_impl = opts.message_type.as_ref().map(|mt| {
        quote! {
            impl ironposh_psrp::ps_value::PsObjectWithType for #name {
                fn message_type(&self) -> ironposh_psrp::MessageType {
                    ironposh_psrp::MessageType::#mt
                }

                fn to_ps_object(&self) -> ironposh_psrp::ps_value::PsValue {
                    ironposh_psrp::ps_value::PsValue::Object(ironposh_psrp::ps_value::ComplexObject::from(self))
                }
            }
        }
    });

    // Dictionary-body mode: serialize fields as a `<DCT>` keyed by field name.
    let dict_inserts: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let prop = &f.name;
            let conv = |inner: TokenStream2| {
                f.with.as_ref().map_or_else(
                    || quote! { ironposh_psrp::ps_value::ToPsValue::to_ps_value(#inner) },
                    |w| quote! { #w::to_ps_value(#inner) },
                )
            };
            if f.flatten {
                return quote! {
                    for (__k, __v) in &value.#ident {
                        __entries.insert(
                            ironposh_psrp::ps_value::PsValue::Primitive(
                                ironposh_psrp::ps_value::PsPrimitiveValue::Str(__k.clone())
                            ),
                            ironposh_psrp::ps_value::ToPsValue::to_ps_value(__v),
                        );
                    }
                };
            }
            let key = quote! {
                ironposh_psrp::ps_value::PsValue::Primitive(
                    ironposh_psrp::ps_value::PsPrimitiveValue::Str(#prop.to_string())
                )
            };
            if f.is_option {
                let v = conv(quote! { inner });
                quote! {
                    if let ::core::option::Option::Some(inner) = &value.#ident {
                        __entries.insert(#key, #v);
                    }
                }
            } else {
                let v = conv(quote! { &value.#ident });
                quote! { __entries.insert(#key, #v); }
            }
        })
        .collect();
    // value_dictionary mode: each field → integer key, value wrapped in `{T, V}`.
    let vdict_inserts: Vec<TokenStream2> = if opts.value_dictionary {
        fields
            .iter()
            .map(|f| {
                let ident = &f.ident;
                let key = f.key.unwrap_or_else(|| {
                    panic!(
                        "#[ps(value_dictionary)] field `{}` needs #[ps(key = N)]",
                        f.name
                    )
                });
                let tag = f.type_tag.clone().unwrap_or_else(|| {
                    panic!(
                        "#[ps(value_dictionary)] field `{}` needs #[ps(type_tag = \"..\")]",
                        f.name
                    )
                });
                // Honor a custom `with` converter for the wrapped value, falling
                // back to the `ToPsValue` trait.
                let v = f.with.as_ref().map_or_else(
                    || quote! { ironposh_psrp::ps_value::ToPsValue::to_ps_value(&value.#ident) },
                    |w| quote! { #w::to_ps_value(&value.#ident) },
                );
                quote! {
                    {
                        let __w = ironposh_psrp::ps_value::ComplexObject::standard()
                            .extended("T", #tag)
                            .extended("V", #v)
                            .build();
                        __entries.insert(
                            ironposh_psrp::ps_value::PsValue::Primitive(
                                ironposh_psrp::ps_value::PsPrimitiveValue::I32(#key)
                            ),
                            ironposh_psrp::ps_value::PsValue::Object(__w),
                        );
                    }
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    let dict_tns = &opts.type_names;
    let from_body = if opts.value_dictionary {
        quote! {
            let mut __entries: ::std::collections::BTreeMap<
                ironposh_psrp::ps_value::PsValue,
                ironposh_psrp::ps_value::PsValue,
            > = ::core::default::Default::default();
            #(#vdict_inserts)*
            ironposh_psrp::ps_value::ComplexObject {
                type_def: ::core::option::Option::Some(ironposh_psrp::ps_value::PsType {
                    type_names: ::std::vec![ #( ::std::borrow::Cow::Borrowed(#dict_tns) ),* ],
                }),
                to_string: ::core::option::Option::None,
                content: ironposh_psrp::ps_value::ComplexObjectContent::Container(
                    ironposh_psrp::ps_value::Container::Dictionary(__entries)
                ),
                properties: ironposh_psrp::ps_value::Properties::new(),
            }
        }
    } else if opts.dictionary {
        quote! {
            let mut __entries: ::std::collections::BTreeMap<
                ironposh_psrp::ps_value::PsValue,
                ironposh_psrp::ps_value::PsValue,
            > = ::core::default::Default::default();
            #(#dict_inserts)*
            ironposh_psrp::ps_value::ComplexObject {
                type_def: ::core::option::Option::Some(ironposh_psrp::ps_value::PsType {
                    type_names: ::std::vec![ #( ::std::borrow::Cow::Borrowed(#dict_tns) ),* ],
                }),
                to_string: ::core::option::Option::None,
                content: ironposh_psrp::ps_value::ComplexObjectContent::Container(
                    ironposh_psrp::ps_value::Container::Dictionary(__entries)
                ),
                properties: ironposh_psrp::ps_value::Properties::new(),
            }
        }
    } else {
        quote! {
            let mut obj = ironposh_psrp::ps_value::ComplexObject::standard();
            #type_names_setup
            #(#inserts)*
            obj.build()
        }
    };

    Ok(quote! {
        #message_impl

        impl ::core::convert::From<&#name> for ironposh_psrp::ps_value::ComplexObject {
            fn from(value: &#name) -> Self {
                #from_body
            }
        }

        impl ::core::convert::From<#name> for ironposh_psrp::ps_value::ComplexObject {
            fn from(value: #name) -> Self {
                Self::from(&value)
            }
        }

        // Nesting bridge: lets a field of this type be (de)serialized as a
        // nested `<Obj>` inside another derived struct.
        impl ironposh_psrp::ps_value::ToPsValue for #name {
            fn to_ps_value(&self) -> ironposh_psrp::ps_value::PsValue {
                ironposh_psrp::ps_value::PsValue::Object(ironposh_psrp::ps_value::ComplexObject::from(self))
            }
        }
    })
}

fn impl_ps_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let opts = ps_struct_opts(input)?;
    let fields = ps_named_fields(input)?;

    // A present-but-`Nil` property is semantically the same as an absent one:
    // PowerShell emits `<Nil/>` for null/empty members. `Option`/`default`
    // fields must treat it like a missing property (→ `None`/`Default`), exactly
    // as the L1 `ComplexObject::opt` accessor does. Without this, a `default`
    // `String` field arriving as `Nil` (e.g. ProgressRecord's `CurrentOperation`)
    // fails its `from_ps_value` and the whole host call is rejected.
    let is_nil = |bind: &TokenStream2| {
        quote! {
            ::core::matches!(
                #bind,
                ironposh_psrp::ps_value::PsValue::Primitive(
                    ironposh_psrp::ps_value::PsPrimitiveValue::Nil
                )
            )
        }
    };

    // Dictionary-body mode: read fields from a `<DCT>` keyed by field name.
    let dict_assignments: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let prop = &f.name;
            let key = quote! {
                &ironposh_psrp::ps_value::PsValue::Primitive(
                    ironposh_psrp::ps_value::PsPrimitiveValue::Str(#prop.to_string())
                )
            };
            if f.flatten {
                return quote! {
                    #ident: __dict.iter().filter_map(|(__k, __v)| match __k {
                        ironposh_psrp::ps_value::PsValue::Primitive(
                            ironposh_psrp::ps_value::PsPrimitiveValue::Str(__s)
                        ) if !__named.contains(&__s.as_str()) => {
                            ::core::option::Option::Some((__s.clone(), __v.clone()))
                        }
                        _ => ::core::option::Option::None,
                    }).collect()
                };
            }
            let conv = |v: TokenStream2| {
                f.with.as_ref().map_or_else(
                    || quote! { ironposh_psrp::ps_value::FromPsValue::from_ps_value(#v)? },
                    |w| quote! { #w::from_ps_value(#v)? },
                )
            };
            let v_is_nil = is_nil(&quote! { v });
            if f.is_option {
                let c = conv(quote! { v });
                quote! {
                    #ident: match __dict.get(#key) {
                        ::core::option::Option::Some(v) if !#v_is_nil =>
                            ::core::option::Option::Some(#c),
                        _ => ::core::option::Option::None,
                    }
                }
            } else if f.default {
                let c = conv(quote! { v });
                quote! {
                    #ident: match __dict.get(#key) {
                        ::core::option::Option::Some(v) if !#v_is_nil => #c,
                        _ => ::core::default::Default::default(),
                    }
                }
            } else {
                let got = quote! {
                    __dict.get(#key).ok_or_else(|| {
                        ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("Missing dictionary key: {}", #prop)
                        )
                    })?
                };
                let c = conv(got);
                quote! { #ident: #c }
            }
        })
        .collect();

    let assignments: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let prop = &f.name;

            // `wrap`: descend one level into the named sub-property before converting.
            if let Some(wrapname) = &f.wrap {
                let missing = if f.default {
                    quote! { ::core::default::Default::default() }
                } else {
                    quote! {
                        return ::core::result::Result::Err(
                            ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                                ::std::format!("Missing property: {}", #prop)
                            )
                        )
                    }
                };
                return quote! {
                    #ident: match value.get_property(#prop) {
                        ::core::option::Option::Some(
                            ironposh_psrp::ps_value::PsValue::Object(__o)
                        ) => match __o.get_property(#wrapname) {
                            ::core::option::Option::Some(__inner) =>
                                ironposh_psrp::ps_value::FromPsValue::from_ps_value(__inner)?,
                            ::core::option::Option::None => #missing,
                        },
                        _ => #missing,
                    }
                };
            }

            // `flatten_prefix`: gather the parent's prefixed properties back into a
            // sub-object and convert it; `None` when no prefixed property exists.
            if let Some(prefix) = &f.flatten_prefix {
                return quote! {
                    #ident: {
                        let mut __sub = ironposh_psrp::ps_value::ComplexObject::standard();
                        let mut __found = false;
                        for (__n, __p) in value.properties.iter() {
                            if let ::core::option::Option::Some(__stripped) =
                                __n.strip_prefix(#prefix)
                            {
                                __sub = match __p.kind {
                                    ironposh_psrp::ps_value::PropertyKind::Adapted =>
                                        __sub.adapted(__stripped.to_string(), __p.value.clone()),
                                    ironposh_psrp::ps_value::PropertyKind::Extended =>
                                        __sub.extended(__stripped.to_string(), __p.value.clone()),
                                };
                                __found = true;
                            }
                        }
                        if __found {
                            ::core::option::Option::Some(
                                ironposh_psrp::ps_value::FromPsValue::from_ps_value(
                                    &ironposh_psrp::ps_value::PsValue::Object(__sub.build())
                                )?
                            )
                        } else {
                            ::core::option::Option::None
                        }
                    }
                };
            }

            // Fast path: single name, no custom converter, no default — use L1
            // accessors (precise error messages).
            if f.also.is_empty() && f.with.is_none() && !f.default && f.fallback_object.is_none() {
                return if f.is_option {
                    quote! { #ident: value.opt(#prop)? }
                } else {
                    quote! { #ident: value.req(#prop)? }
                };
            }

            // General path: look up the primary name, then any `also` aliases,
            // then (if `fallback_object` is set) the same names inside that
            // sibling object.
            let also = &f.also;
            let nested_lookup = f.fallback_object.as_ref().map(|obj_name| {
                quote! {
                    .or_else(|| value.get_property(#obj_name).and_then(|__fo| match __fo {
                        ironposh_psrp::ps_value::PsValue::Object(__fobj) =>
                            __fobj.get_property(#prop)
                                #( .or_else(|| __fobj.get_property(#also)) )*,
                        ironposh_psrp::ps_value::PsValue::Primitive(_) =>
                            ::core::option::Option::None,
                    }))
                }
            });
            let lookup = quote! {
                value.get_property(#prop)
                    #( .or_else(|| value.get_property(#also)) )*
                    #nested_lookup
            };
            let convert = |v: TokenStream2| {
                f.with.as_ref().map_or_else(
                    || quote! { ironposh_psrp::ps_value::FromPsValue::from_ps_value(#v)? },
                    |with| quote! { #with::from_ps_value(#v)? },
                )
            };
            let v_is_nil = is_nil(&quote! { v });
            if f.is_option {
                let conv = convert(quote! { v });
                quote! {
                    #ident: match #lookup {
                        ::core::option::Option::Some(v) if !#v_is_nil =>
                            ::core::option::Option::Some(#conv),
                        _ => ::core::option::Option::None,
                    }
                }
            } else if f.default {
                let conv = convert(quote! { v });
                quote! {
                    #ident: match #lookup {
                        ::core::option::Option::Some(v) if !#v_is_nil => #conv,
                        _ => ::core::default::Default::default(),
                    }
                }
            } else {
                let got = quote! {
                    #lookup.ok_or_else(|| {
                        ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("Missing property: {}", #prop)
                        )
                    })?
                };
                let conv = convert(got);
                quote! { #ident: #conv }
            }
        })
        .collect();

    // value_dictionary mode: read each field from its integer key, unwrapping `V`.
    let vdict_assignments: Vec<TokenStream2> =
        if opts.value_dictionary {
            fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let key = f.key.unwrap_or_else(|| {
                panic!("#[ps(value_dictionary)] field `{}` needs #[ps(key = N)]", f.name)
            });
            // Honor a custom `with` converter for the wrapped value.
            let conv = f.with.as_ref().map_or_else(
                || quote! { ironposh_psrp::ps_value::FromPsValue::from_ps_value(__v)? },
                |w| quote! { #w::from_ps_value(__v)? },
            );
            quote! {
                #ident: {
                    let __wv = __dict
                        .get(&ironposh_psrp::ps_value::PsValue::Primitive(
                            ironposh_psrp::ps_value::PsPrimitiveValue::I32(#key)
                        ))
                        .ok_or_else(|| ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("Missing host data key {}", #key)
                        ))?;
                    let __wobj = match __wv {
                        ironposh_psrp::ps_value::PsValue::Object(o) => o,
                        _ => return ::core::result::Result::Err(
                            ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                                ::std::format!("host data key {} is not an object", #key)
                            )
                        ),
                    };
                    let __v = __wobj.get_property("V").ok_or_else(|| {
                        ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("host data key {} missing V", #key)
                        )
                    })?;
                    #conv
                }
            }
        })
        .collect()
        } else {
            Vec::new()
        };

    // Names claimed by non-flatten fields (so a `flatten` field can collect the rest).
    let claimed_names: Vec<&String> = fields
        .iter()
        .filter(|f| !f.flatten)
        .map(|f| &f.name)
        .collect();
    let try_from_body = if opts.value_dictionary {
        quote! {
            let __dict = match &value.content {
                ironposh_psrp::ps_value::ComplexObjectContent::Container(
                    ironposh_psrp::ps_value::Container::Dictionary(d)
                ) => d,
                _ => return ::core::result::Result::Err(
                    ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                        ::std::format!("expected a dictionary for {}", ::core::stringify!(#name))
                    )
                ),
            };
            ::core::result::Result::Ok(Self { #(#vdict_assignments),* })
        }
    } else if opts.dictionary {
        quote! {
            let __dict = match &value.content {
                ironposh_psrp::ps_value::ComplexObjectContent::Container(
                    ironposh_psrp::ps_value::Container::Dictionary(d)
                ) => d,
                _ => return ::core::result::Result::Err(
                    ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                        ::std::format!("expected a dictionary for {}", ::core::stringify!(#name))
                    )
                ),
            };
            let __named: &[&str] = &[ #(#claimed_names),* ];
            ::core::result::Result::Ok(Self { #(#dict_assignments),* })
        }
    } else {
        quote! { ::core::result::Result::Ok(Self { #(#assignments),* }) }
    };

    Ok(quote! {
        impl ::core::convert::TryFrom<ironposh_psrp::ps_value::ComplexObject> for #name {
            type Error = ironposh_psrp::PowerShellRemotingError;

            fn try_from(value: ironposh_psrp::ps_value::ComplexObject) -> ::core::result::Result<Self, Self::Error> {
                #try_from_body
            }
        }

        // Nesting bridge: lets a field of this type be deserialized from a
        // nested `<Obj>` inside another derived struct.
        impl ironposh_psrp::ps_value::FromPsValue for #name {
            const TYPE_LABEL: &'static str = ::core::stringify!(#name);

            fn from_ps_value(
                value: &ironposh_psrp::ps_value::PsValue,
            ) -> ::core::result::Result<Self, ironposh_psrp::PowerShellRemotingError> {
                match value {
                    ironposh_psrp::ps_value::PsValue::Object(obj) => {
                        <Self as ::core::convert::TryFrom<ironposh_psrp::ps_value::ComplexObject>>::try_from(obj.clone())
                    }
                    ironposh_psrp::ps_value::PsValue::Primitive(_) => {
                        ::core::result::Result::Err(ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("expected {} object", ::core::stringify!(#name))
                        ))
                    }
                }
            }
        }
    })
}

/// Derives the CLIXML representation of a fieldless Rust enum (RFC #12, L3).
///
/// Two wire encodings, chosen by `#[ps(repr = ..)]`:
/// - `"object"` (default): a full enum `<Obj>` — a `<TN>` type-name chain, a
///   `<ToString>` of the variant name, and the discriminant as `<I32>` content.
///   Requires `#[ps(type_names("A","B",..))]`.
/// - `"i32"`: a bare `<I32>` primitive (the variant's discriminant).
///
/// Each variant must be unit and carry an explicit discriminant (`= N`).
/// `#[ps(rename = "..")]` overrides the `<ToString>` name for a variant.
/// Generates `ToPsValue`/`FromPsValue` (+ `From`/`TryFrom<ComplexObject>` for
/// the object repr), so the enum composes inside derived structs.
#[proc_macro_derive(PsEnum, attributes(ps))]
pub fn derive_ps_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_ps_enum(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

enum EnumRepr {
    Object,
    I32,
}

struct PsEnumVariant {
    ident: Ident,
    name: String,
    disc: syn::Expr,
}

fn impl_ps_enum(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "PsEnum can only be derived for enums",
        ));
    };

    let mut repr = EnumRepr::Object;
    let mut type_names: Vec<String> = Vec::new();
    for attr in &input.attrs {
        if !attr.path().is_ident("ps") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("repr") {
                let lit: LitStr = meta.value()?.parse()?;
                repr = match lit.value().as_str() {
                    "object" => EnumRepr::Object,
                    "i32" => EnumRepr::I32,
                    other => return Err(meta.error(format!("unknown #[ps(repr = ..)]: {other}"))),
                };
            } else if meta.path.is_ident("type_names") {
                let content;
                syn::parenthesized!(content in meta.input);
                let names = content
                    .parse_terminated(<LitStr as syn::parse::Parse>::parse, syn::Token![,])?;
                for l in names {
                    type_names.push(l.value());
                }
            } else {
                return Err(meta.error("unknown #[ps(..)] enum attribute"));
            }
            Ok(())
        })?;
    }

    let mut variants = Vec::new();
    for v in &data.variants {
        if !matches!(v.fields, Fields::Unit) {
            return Err(syn::Error::new_spanned(
                v,
                "PsEnum variants must be unit (fieldless)",
            ));
        }
        let disc = v
            .discriminant
            .as_ref()
            .map(|(_, e)| e.clone())
            .ok_or_else(|| {
                syn::Error::new_spanned(v, "PsEnum variants need an explicit discriminant (= N)")
            })?;
        let mut vname = v.ident.to_string();
        for attr in &v.attrs {
            if !attr.path().is_ident("ps") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("rename") {
                    let lit: LitStr = meta.value()?.parse()?;
                    vname = lit.value();
                } else {
                    return Err(meta.error("unknown #[ps(..)] variant attribute"));
                }
                Ok(())
            })?;
        }
        variants.push(PsEnumVariant {
            ident: v.ident.clone(),
            name: vname,
            disc,
        });
    }

    let idents: Vec<&Ident> = variants.iter().map(|v| &v.ident).collect();
    let vnames: Vec<&String> = variants.iter().map(|v| &v.name).collect();
    let discs: Vec<&syn::Expr> = variants.iter().map(|v| &v.disc).collect();

    // i32 -> variant, in an inherent method. PsEnum deliberately does NOT
    // implement `TryFrom` (whose `Error` associated item would collide with an
    // `Error` *variant* — rust#57644); with no such associated item in scope,
    // `Self::Variant` paths are unambiguous.
    let from_disc_impl = quote! {
        impl #name {
            #[doc(hidden)]
            fn __ps_from_discriminant(v: i32) -> ::core::option::Option<Self> {
                #( if v == #discs { return ::core::option::Option::Some(Self::#idents); } )*
                ::core::option::Option::None
            }
        }
    };
    // Expression: map `v: i32` to `Result<Self, _>` via the inherent method.
    let from_i32 = quote! {
        #name::__ps_from_discriminant(v).ok_or_else(|| {
            ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                ::std::format!("invalid {} enum value: {}", ::core::stringify!(#name), v)
            )
        })
    };

    match repr {
        EnumRepr::Object => {
            if type_names.is_empty() {
                return Err(syn::Error::new_spanned(
                    input,
                    "PsEnum with repr = \"object\" requires #[ps(type_names(..))]",
                ));
            }
            Ok(quote! {
                #from_disc_impl

                impl ::core::convert::From<&#name> for ironposh_psrp::ps_value::ComplexObject {
                    fn from(value: &#name) -> Self {
                        let (val, name): (i32, &'static str) = match value {
                            #( #name::#idents => (#discs, #vnames) ),*
                        };
                        ironposh_psrp::ps_value::ComplexObject {
                            type_def: ::core::option::Option::Some(ironposh_psrp::ps_value::PsType {
                                type_names: ::std::vec![ #( ::std::borrow::Cow::Borrowed(#type_names) ),* ],
                            }),
                            to_string: ::core::option::Option::Some(::std::string::ToString::to_string(name)),
                            content: ironposh_psrp::ps_value::ComplexObjectContent::PsEnums(
                                ironposh_psrp::ps_value::PsEnums { value: val }
                            ),
                            properties: ironposh_psrp::ps_value::Properties::new(),
                        }
                    }
                }

                impl ::core::convert::From<#name> for ironposh_psrp::ps_value::ComplexObject {
                    fn from(value: #name) -> Self { Self::from(&value) }
                }

                impl ironposh_psrp::ps_value::ToPsValue for #name {
                    fn to_ps_value(&self) -> ironposh_psrp::ps_value::PsValue {
                        ironposh_psrp::ps_value::PsValue::Object(ironposh_psrp::ps_value::ComplexObject::from(self))
                    }
                }

                impl #name {
                    /// Parse this enum from its CLIXML enum-`<Obj>` form.
                    pub fn from_ps_object(
                        obj: ironposh_psrp::ps_value::ComplexObject,
                    ) -> ::core::result::Result<Self, ironposh_psrp::PowerShellRemotingError> {
                        let v: i32 = match &obj.content {
                            ironposh_psrp::ps_value::ComplexObjectContent::PsEnums(e) => e.value,
                            ironposh_psrp::ps_value::ComplexObjectContent::ExtendedPrimitive(
                                ironposh_psrp::ps_value::PsPrimitiveValue::I32(i)
                            ) => *i,
                            _ => return ::core::result::Result::Err(
                                ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                                    ::std::format!("{} must be an enum object", ::core::stringify!(#name))
                                )
                            ),
                        };
                        #from_i32
                    }
                }

                impl ironposh_psrp::ps_value::FromPsValue for #name {
                    const TYPE_LABEL: &'static str = ::core::stringify!(#name);
                    fn from_ps_value(
                        value: &ironposh_psrp::ps_value::PsValue,
                    ) -> ::core::result::Result<Self, ironposh_psrp::PowerShellRemotingError> {
                        match value {
                            ironposh_psrp::ps_value::PsValue::Object(o) => Self::from_ps_object(o.clone()),
                            ironposh_psrp::ps_value::PsValue::Primitive(
                                ironposh_psrp::ps_value::PsPrimitiveValue::I32(i)
                            ) => { let v = *i; #from_i32 }
                            _ => ::core::result::Result::Err(
                                ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                                    ::std::format!("expected {} enum", ::core::stringify!(#name))
                                )
                            ),
                        }
                    }
                }
            })
        }
        EnumRepr::I32 => Ok(quote! {
            #from_disc_impl

            impl ironposh_psrp::ps_value::ToPsValue for #name {
                fn to_ps_value(&self) -> ironposh_psrp::ps_value::PsValue {
                    let val: i32 = match self { #( #name::#idents => #discs ),* };
                    ironposh_psrp::ps_value::PsValue::Primitive(ironposh_psrp::ps_value::PsPrimitiveValue::I32(val))
                }
            }

            impl ironposh_psrp::ps_value::FromPsValue for #name {
                const TYPE_LABEL: &'static str = ::core::stringify!(#name);
                fn from_ps_value(
                    value: &ironposh_psrp::ps_value::PsValue,
                ) -> ::core::result::Result<Self, ironposh_psrp::PowerShellRemotingError> {
                    match value {
                        ironposh_psrp::ps_value::PsValue::Primitive(
                            ironposh_psrp::ps_value::PsPrimitiveValue::I32(i)
                        ) => { let v = *i; #from_i32 }
                        _ => ::core::result::Result::Err(
                            ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                                ::std::format!("expected I32 for {}", ::core::stringify!(#name))
                            )
                        ),
                    }
                }
            }
        }),
    }
}

/// Derives `ToPsValue`/`FromPsValue` for an *untagged* polymorphic enum (RFC #12).
///
/// Each variant must be a single-field newtype `Variant(T)` whose inner type
/// already (de)serializes. Serialize delegates to the active variant's inner
/// `ToPsValue`. Deserialize dispatches by wire shape, in declaration order:
/// - `#[ps(primitive)]`: matches any `PsValue::Primitive`.
/// - `#[ps(type_match = "Substr")]`: matches a `PsValue::Object` whose `<TN>`
///   chain contains `Substr`.
/// - `#[ps(fallback)]`: matches anything not yet claimed (inner is usually
///   `PsValue`, the dynamic escape hatch for arbitrary remote objects).
#[proc_macro_derive(PsUnion, attributes(ps))]
pub fn derive_ps_union(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match impl_ps_union(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

struct PsUnionVariant {
    ident: Ident,
    primitive: bool,
    type_match: Option<String>,
    fallback: bool,
}

fn impl_ps_union(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            input,
            "PsUnion can only be derived for enums",
        ));
    };

    let mut variants = Vec::new();
    for v in &data.variants {
        if !matches!(&v.fields, Fields::Unnamed(f) if f.unnamed.len() == 1) {
            return Err(syn::Error::new_spanned(
                v,
                "PsUnion variants must be single-field newtypes: Variant(T)",
            ));
        }
        let mut primitive = false;
        let mut type_match = None;
        let mut fallback = false;
        for attr in &v.attrs {
            if !attr.path().is_ident("ps") {
                continue;
            }
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("primitive") {
                    primitive = true;
                } else if meta.path.is_ident("fallback") {
                    fallback = true;
                } else if meta.path.is_ident("type_match") {
                    let lit: LitStr = meta.value()?.parse()?;
                    type_match = Some(lit.value());
                } else {
                    return Err(meta.error("unknown #[ps(..)] PsUnion variant attribute"));
                }
                Ok(())
            })?;
        }
        // Each variant needs exactly one dispatch mode, or deserialize would
        // silently never select it (serialize-only round-trip asymmetry).
        if usize::from(primitive) + usize::from(fallback) + usize::from(type_match.is_some()) != 1 {
            return Err(syn::Error::new_spanned(
                v,
                "PsUnion variant needs exactly one of #[ps(primitive)], \
                 #[ps(type_match = \"..\")], or #[ps(fallback)]",
            ));
        }
        variants.push(PsUnionVariant {
            ident: v.ident.clone(),
            primitive,
            type_match,
            fallback,
        });
    }

    let to_arms: Vec<TokenStream2> = variants
        .iter()
        .map(|v| {
            let id = &v.ident;
            quote! {
                #name::#id(__inner) => ironposh_psrp::ps_value::ToPsValue::to_ps_value(__inner),
            }
        })
        .collect();

    // Deserialize dispatch, in declaration order.
    let primitive_arm = variants.iter().find(|v| v.primitive).map(|v| {
        let id = &v.ident;
        quote! {
            if let ironposh_psrp::ps_value::PsValue::Primitive(_) = value {
                return ::core::result::Result::Ok(
                    #name::#id(ironposh_psrp::ps_value::FromPsValue::from_ps_value(value)?)
                );
            }
        }
    });
    let type_match_arms: Vec<TokenStream2> = variants
        .iter()
        .filter_map(|v| {
            v.type_match.as_ref().map(|needle| {
                let id = &v.ident;
                quote! {
                    if let ironposh_psrp::ps_value::PsValue::Object(__o) = value {
                        if __o.type_def.as_ref().is_some_and(|__t| {
                            __t.type_names.iter().any(|__n| __n.contains(#needle))
                        }) {
                            return ::core::result::Result::Ok(
                                #name::#id(ironposh_psrp::ps_value::FromPsValue::from_ps_value(value)?)
                            );
                        }
                    }
                }
            })
        })
        .collect();
    let fallback_expr = variants.iter().find(|v| v.fallback).map_or_else(
        || {
            quote! {
                ::core::result::Result::Err(
                    ironposh_psrp::PowerShellRemotingError::InvalidMessage(
                        ::std::format!("no PsUnion variant of {} matched", ::core::stringify!(#name))
                    )
                )
            }
        },
        |v| {
            let id = &v.ident;
            quote! {
                ::core::result::Result::Ok(
                    #name::#id(ironposh_psrp::ps_value::FromPsValue::from_ps_value(value)?)
                )
            }
        },
    );

    Ok(quote! {
        impl ironposh_psrp::ps_value::ToPsValue for #name {
            fn to_ps_value(&self) -> ironposh_psrp::ps_value::PsValue {
                match self { #(#to_arms)* }
            }
        }

        impl ironposh_psrp::ps_value::FromPsValue for #name {
            const TYPE_LABEL: &'static str = ::core::stringify!(#name);
            fn from_ps_value(
                value: &ironposh_psrp::ps_value::PsValue,
            ) -> ::core::result::Result<Self, ironposh_psrp::PowerShellRemotingError> {
                #primitive_arm
                #(#type_match_arms)*
                #fallback_expr
            }
        }
    })
}

/// XML element's children.
#[proc_macro_derive(SimpleTagValue)]
pub fn derive_simple_tag_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = impl_simple_tag_value(&input);
    TokenStream::from(expanded)
}

/// Derives XmlDeserialize implementation for structs where all fields are `Option<Tag<'a, ValueType, TagName>>`
///
/// This macro assumes that all fields in the struct are optional Tag fields and generates
/// a complete XmlDeserialize implementation with visitor pattern that can parse XML nodes
/// into the struct by matching tag names to field names.
#[proc_macro_derive(SimpleXmlDeserialize)]
pub fn derive_simple_xml_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let expanded = impl_simple_xml_deserialize(&input);
    TokenStream::from(expanded)
}

/// Derives [`ironposh_xml::mapping::FromXml`] for a WinRM struct whose fields
/// are `Tag<'a, V, N>` / `Option<Tag<'a, V, N>>`.
///
/// Generates a direct, namespace-correct `from_xml(node)` — no visitor struct,
/// no `finish()`. Each child element is matched by its `(namespace-URI,
/// local-name)` pair, sourced from the field's `N: TagName` (`NAMESPACE` +
/// `TAG_NAME`); the prefix is never compared. `Option<_>` fields stay `None`
/// when absent; required fields error. This is the deserialize-side replacement
/// for the visitor that `SimpleXmlDeserialize` generates.
#[proc_macro_derive(FromXml)]
pub fn derive_from_xml(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TokenStream::from(impl_from_xml(&input))
}

fn impl_from_xml(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("FromXml can only be derived for structs with named fields"),
        },
        _ => panic!("FromXml can only be derived for structs"),
    };

    let entries: Vec<SimpleFieldEntry> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().clone();
            let field_type = field.ty.clone();
            let is_optional = is_option_type(&field_type);
            let tag_name_type = extract_tag_name_type(&field_type);
            SimpleFieldEntry {
                field_name,
                field_type,
                tag_name_type,
                is_optional,
            }
        })
        .collect();

    let inits = entries.iter().map(|e| {
        let f = &e.field_name;
        quote! { let mut #f = None; }
    });

    // One namespace-correct match per field: identity is (URI, local-name).
    let matchers = entries.iter().filter_map(|e| {
        let f = &e.field_name;
        e.tag_name_type.as_ref().map(|n| {
            quote! {
                if child.is_element_named(
                    <crate::cores::#n as crate::cores::TagName>::NAMESPACE,
                    <crate::cores::#n as crate::cores::TagName>::TAG_NAME,
                ) {
                    #f = Some(ironposh_xml::parser::XmlDeserialize::from_node(child)?);
                    continue;
                }
            }
        })
    });

    let construct = entries.iter().map(|e| {
        let f = &e.field_name;
        if e.is_optional {
            quote! { #f }
        } else {
            quote! {
                #f: #f.ok_or_else(|| ironposh_xml::XmlError::InvalidXml(
                    format!("Missing {} in {}", stringify!(#f), stringify!(#name))
                ))?
            }
        }
    });

    quote! {
        impl #impl_generics ironposh_xml::mapping::FromXml<'a> for #name #ty_generics #where_clause {
            fn from_xml(
                node: ironposh_xml::parser::Node<'a, 'a>,
            ) -> Result<Self, ironposh_xml::XmlError> {
                use ironposh_xml::mapping::NodeExt;
                #(#inits)*
                for child in node.children() {
                    if !child.is_element() {
                        continue;
                    }
                    #(#matchers)*
                }
                Ok(#name { #(#construct),* })
            }
        }
    }
}

fn impl_simple_tag_value(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("SimpleTagValue can only be derived for structs with named fields"),
        },
        _ => panic!("SimpleTagValue can only be derived for structs"),
    };

    // Classify fields as required (Tag<..>) or optional (Option<Tag<..>>)
    let field_info: Vec<FieldInfo> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let is_optional = is_option_type(&field.ty);
            FieldInfo {
                name: field_name,
                is_optional,
            }
        })
        .collect();

    let field_list = {
        let field_names: Vec<&Ident> = field_info.iter().map(|f| f.name).collect();
        quote! { #(#field_names),* }
    };

    // Generate code for each field based on whether it's optional or required
    let field_additions: Vec<TokenStream2> = field_info
        .iter()
        .map(|field| {
            let field_name = field.name;
            if field.is_optional {
                quote! {
                    if let Some(tag) = #field_name {
                        array.push(tag.into_element());
                    }
                }
            } else {
                quote! {
                    array.push(#field_name.into_element());
                }
            }
        })
        .collect();

    quote! {
        impl #impl_generics crate::cores::TagValue<'a> for #name #ty_generics #where_clause {
            fn append_to_element(self, element: ironposh_xml::builder::Element<'a>) -> ironposh_xml::builder::Element<'a> {
                let Self { #field_list } = self;

                let mut array = Vec::new();

                #(#field_additions)*

                element.add_children(array)
            }
        }
    }
}

fn impl_simple_xml_deserialize(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;
    let generics = &input.generics;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("RegularXmlDeserialize can only be derived for structs with named fields"),
        },
        _ => panic!("RegularXmlDeserialize can only be derived for structs"),
    };

    let visitor_name = format_ident!("{}Visitor", name);

    // Extract field information - handle both Tag<..> and Option<Tag<..>>
    let field_entries: Vec<SimpleFieldEntry> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap().clone();
            let field_type = field.ty.clone();
            let is_optional = is_option_type(&field_type);
            let tag_name_type = extract_tag_name_type(&field_type);

            SimpleFieldEntry {
                field_name,
                field_type,
                tag_name_type,
                is_optional,
            }
        })
        .collect();

    // Generate Visitor struct
    let visitor_struct = generate_simple_visitor_struct(&visitor_name, generics, &field_entries);

    // Generate XmlVisitor implementation
    let xml_visitor_impl =
        generate_simple_xml_visitor_impl(&visitor_name, name, generics, &field_entries);

    // Generate XmlDeserialize implementation
    let xml_deserialize_impl = generate_xml_deserialize_impl(name, &visitor_name, generics);

    quote! {
        #visitor_struct
        #xml_visitor_impl
        #xml_deserialize_impl
    }
}

struct SimpleFieldEntry {
    field_name: Ident,
    field_type: Type,
    tag_name_type: Option<Ident>,
    is_optional: bool,
}

struct FieldInfo<'a> {
    name: &'a Ident,
    is_optional: bool,
}

fn generate_simple_visitor_struct(
    visitor_name: &Ident,
    generics: &Generics,
    field_entries: &[SimpleFieldEntry],
) -> TokenStream2 {
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();

    let visitor_fields: Vec<TokenStream2> = field_entries
        .iter()
        .map(|entry| {
            let field_name = &entry.field_name;
            let field_type = &entry.field_type;
            if entry.is_optional {
                // Optional fields stay as Option<Tag<..>> in visitor
                quote! { pub #field_name: #field_type }
            } else {
                // Required fields are stored as Option<Tag<..>> during parsing, then validated
                quote! { pub #field_name: Option<#field_type> }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #visitor_name #impl_generics #where_clause {
            #(#visitor_fields),*
        }
    }
}

fn generate_simple_xml_visitor_impl(
    visitor_name: &Ident,
    struct_name: &Ident,
    generics: &Generics,
    field_entries: &[SimpleFieldEntry],
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Generate match arms for each field
    let match_arms: Vec<TokenStream2> = field_entries
        .iter()
        .filter_map(|entry| {
            entry.tag_name_type.as_ref().map(|tag_name_type| {
        let field_name = &entry.field_name;
        quote! {
            crate::cores::#tag_name_type::TAG_NAME => {
                self.#field_name = Some(ironposh_xml::parser::XmlDeserialize::from_node(child)?);
            }
        }
    })
        })
        .collect();

    // Generate field list for finish method
    let field_names: Vec<&Ident> = field_entries.iter().map(|f| &f.field_name).collect();
    let field_list = quote! { #(#field_names),* };

    // Separate required and optional fields for finish method
    let required_fields: Vec<&SimpleFieldEntry> =
        field_entries.iter().filter(|f| !f.is_optional).collect();
    let _optional_fields: Vec<&SimpleFieldEntry> =
        field_entries.iter().filter(|f| f.is_optional).collect();

    // Generate required field checks - use different variable names to avoid conflicts
    let required_field_checks: Vec<TokenStream2> = required_fields
        .iter()
        .map(|entry| {
            let field_name = &entry.field_name;
            let checked_field_name = format_ident!("{}_checked", field_name);
            quote! {
                let #checked_field_name = #field_name.ok_or_else(|| {
                    ironposh_xml::XmlError::InvalidXml(format!(
                        "Missing {} in {}",
                        stringify!(#field_name),
                        stringify!(#struct_name)
                    ))
                })?;
            }
        })
        .collect();

    // Generate field assignments for struct construction
    let field_assignments: Vec<TokenStream2> = field_entries
        .iter()
        .map(|entry| {
            let field_name = &entry.field_name;
            if entry.is_optional {
                // Optional fields use their own value
                quote! { #field_name }
            } else {
                // Required fields use the checked version
                let checked_field_name = format_ident!("{}_checked", field_name);
                quote! { #field_name: #checked_field_name }
            }
        })
        .collect();

    quote! {
        impl #impl_generics ironposh_xml::parser::XmlVisitor<'a> for #visitor_name #ty_generics #where_clause {
            type Value = #struct_name #ty_generics;

            fn visit_children(
                &mut self,
                children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
            ) -> Result<(), ironposh_xml::XmlError> {
                for child in children {
                    if !child.is_element() {
                        continue; // Skip non-element nodes like text/whitespace
                    }

                    let tag_name = child.tag_name().name();
                    let namespace = child.tag_name().namespace();

                    match tag_name {
                        #(#match_arms)*
                        _ => {
                            // Warn about unknown tags instead of erroring - this allows SOAP faults
                            // with unknown namespaces (like WS-Eventing) to be parsed successfully
                            tracing::warn!(
                                target: "xml_parsing",
                                tag_name = tag_name,
                                namespace = ?namespace,
                                struct_name = stringify!(#struct_name),
                                "Unknown tag encountered during XML parsing, ignoring"
                            );
                        }
                    }
                }

                Ok(())
            }

            fn visit_node(&mut self, node: ironposh_xml::parser::Node<'a, 'a>) -> Result<(), ironposh_xml::XmlError> {
                // Get the children and process them
                let children: Vec<_> = node.children().collect();

                self.visit_children(children.into_iter())?;
                Ok(())
            }

            fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
                let Self { #field_list } = self;

                // Validate required fields
                #(#required_field_checks)*

                Ok(#struct_name {
                    #(#field_assignments),*
                })
            }
        }
    }
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(TypePath { path, .. }) = ty {
        if let Some(segment) = path.segments.first() {
            return segment.ident == "Option";
        }
    }
    false
}

fn extract_tag_name_type(ty: &Type) -> Option<Ident> {
    // Try to extract TagName from Tag<'a, ValueType, TagName> or Option<Tag<'a, ValueType, TagName>>
    if let Type::Path(TypePath { path, .. }) = ty {
        for segment in &path.segments {
            if segment.ident == "Tag" || segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    // For Option<Tag<...>>, we need to look at the inner type
                    for arg in &args.args {
                        if let syn::GenericArgument::Type(inner_type) = arg {
                            if let Some(tag_name) = extract_tag_name_from_tag_type(inner_type) {
                                return Some(tag_name);
                            }
                        }
                    }

                    // For Tag<'a, ValueType, TagName>, the third argument is TagName
                    if segment.ident == "Tag" && args.args.len() >= 3 {
                        if let syn::GenericArgument::Type(Type::Path(TypePath { path, .. })) =
                            &args.args[2]
                        {
                            if let Some(segment) = path.segments.last() {
                                return Some(segment.ident.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn extract_tag_name_from_tag_type(ty: &Type) -> Option<Ident> {
    if let Type::Path(TypePath { path, .. }) = ty {
        for segment in &path.segments {
            if segment.ident == "Tag" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if args.args.len() >= 3 {
                        if let syn::GenericArgument::Type(Type::Path(TypePath { path, .. })) =
                            &args.args[2]
                        {
                            if let Some(segment) = path.segments.last() {
                                return Some(segment.ident.clone());
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

fn generate_xml_deserialize_impl(
    struct_name: &Ident,
    visitor_name: &Ident,
    generics: &Generics,
) -> TokenStream2 {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics ironposh_xml::parser::XmlDeserialize<'a> for #struct_name #ty_generics #where_clause {
            type Visitor = #visitor_name #ty_generics;

            fn visitor() -> Self::Visitor {
                #visitor_name::default()
            }

            fn from_node(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
                ironposh_xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
            }
        }
    }
}
