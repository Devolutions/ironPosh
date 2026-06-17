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
struct PsFieldOpts {
    ident: Ident,
    /// CLIXML property name (defaults to the field name).
    name: String,
    /// Whether the field type is `Option<..>`.
    is_option: bool,
    /// Place in the adapted (`<Props>`) bag instead of extended (`<MS>`).
    adapted: bool,
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
            let mut with = None;

            for attr in &field.attrs {
                if !attr.path().is_ident("ps") {
                    continue;
                }
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        let lit: LitStr = meta.value()?.parse()?;
                        name = lit.value();
                    } else if meta.path.is_ident("adapted") {
                        adapted = true;
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
            let prop = &f.name;
            let bag = if f.adapted {
                quote! { adapted }
            } else {
                quote! { extended }
            };
            match (&f.with, f.is_option) {
                // Custom converter on an optional field: skip when None.
                (Some(with), true) => quote! {
                    if let ::core::option::Option::Some(inner) = &value.#ident {
                        obj = obj.#bag(#prop, #with::to_ps_value(inner));
                    }
                },
                // Custom converter on a required field.
                (Some(with), false) => quote! {
                    obj = obj.#bag(#prop, #with::to_ps_value(&value.#ident));
                },
                // Trait-based optional field (extended only): skip when None.
                (None, true) if !f.adapted => {
                    quote! { obj = obj.extended_opt(#prop, value.#ident.as_ref()); }
                }
                // Trait-based field.
                (None, _) => quote! { obj = obj.#bag(#prop, &value.#ident); },
            }
        })
        .collect();

    // `PsObjectWithType` is only generated for top-level messages (those with a
    // message_type); sub-objects skip it but still get the conversions below.
    let message_impl = opts.message_type.as_ref().map(|mt| {
        quote! {
            impl crate::ps_value::PsObjectWithType for #name {
                fn message_type(&self) -> crate::MessageType {
                    crate::MessageType::#mt
                }

                fn to_ps_object(&self) -> crate::ps_value::PsValue {
                    crate::ps_value::PsValue::Object(crate::ps_value::ComplexObject::from(self))
                }
            }
        }
    });

    Ok(quote! {
        #message_impl

        impl ::core::convert::From<&#name> for crate::ps_value::ComplexObject {
            fn from(value: &#name) -> Self {
                let mut obj = crate::ps_value::ComplexObject::standard();
                #(#inserts)*
                obj.build()
            }
        }

        impl ::core::convert::From<#name> for crate::ps_value::ComplexObject {
            fn from(value: #name) -> Self {
                Self::from(&value)
            }
        }

        // Nesting bridge: lets a field of this type be (de)serialized as a
        // nested `<Obj>` inside another derived struct.
        impl crate::ps_value::ToPsValue for #name {
            fn to_ps_value(&self) -> crate::ps_value::PsValue {
                crate::ps_value::PsValue::Object(crate::ps_value::ComplexObject::from(self))
            }
        }
    })
}

fn impl_ps_deserialize(input: &DeriveInput) -> syn::Result<TokenStream2> {
    let name = &input.ident;
    let fields = ps_named_fields(input)?;

    let assignments: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let prop = &f.name;
            match (&f.with, f.is_option) {
                // Custom converter, optional: absent -> None.
                (Some(with), true) => quote! {
                    #ident: match value.get_property(#prop) {
                        ::core::option::Option::Some(v) => ::core::option::Option::Some(#with::from_ps_value(v)?),
                        ::core::option::Option::None => ::core::option::Option::None,
                    }
                },
                // Custom converter, required.
                (Some(with), false) => quote! {
                    #ident: #with::from_ps_value(
                        value.get_property(#prop).ok_or_else(|| {
                            crate::PowerShellRemotingError::InvalidMessage(
                                ::std::format!("Missing property: {}", #prop)
                            )
                        })?
                    )?
                },
                // Trait-based, via the L1 accessors (precise error messages).
                (None, true) => quote! { #ident: value.opt(#prop)? },
                (None, false) => quote! { #ident: value.req(#prop)? },
            }
        })
        .collect();

    Ok(quote! {
        impl ::core::convert::TryFrom<crate::ps_value::ComplexObject> for #name {
            type Error = crate::PowerShellRemotingError;

            fn try_from(value: crate::ps_value::ComplexObject) -> ::core::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#assignments),*
                })
            }
        }

        // Nesting bridge: lets a field of this type be deserialized from a
        // nested `<Obj>` inside another derived struct.
        impl crate::ps_value::FromPsValue for #name {
            const TYPE_LABEL: &'static str = ::core::stringify!(#name);

            fn from_ps_value(
                value: &crate::ps_value::PsValue,
            ) -> ::core::result::Result<Self, crate::PowerShellRemotingError> {
                match value {
                    crate::ps_value::PsValue::Object(obj) => {
                        <Self as ::core::convert::TryFrom<crate::ps_value::ComplexObject>>::try_from(obj.clone())
                    }
                    crate::ps_value::PsValue::Primitive(_) => {
                        ::core::result::Result::Err(crate::PowerShellRemotingError::InvalidMessage(
                            ::std::format!("expected {} object", ::core::stringify!(#name))
                        ))
                    }
                }
            }
        }
    })
}

/// Derives TagValue implementation for structs where all fields are `Option<Tag<'a, ValueType, TagName>>`
///
/// This macro assumes that all fields in the struct are optional Tag fields and generates
/// a TagValue implementation that converts each Some(tag) to an element and adds it to the
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
