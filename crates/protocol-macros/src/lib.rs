use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{parse_macro_input, Data, DeriveInput, Fields, Generics, Type, TypePath};

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
            fn append_to_element(self, element: xml::builder::Element<'a>) -> xml::builder::Element<'a> {
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
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

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
            if let Some(tag_name_type) = &entry.tag_name_type {
                let field_name = &entry.field_name;
                Some(quote! {
                    crate::cores::#tag_name_type::TAG_NAME => {
                        self.#field_name = Some(xml::parser::XmlDeserialize::from_node(child)?);
                    }
                })
            } else {
                None
            }
        })
        .collect();

    // Generate field list for finish method
    let field_names: Vec<&Ident> = field_entries.iter().map(|f| &f.field_name).collect();
    let field_list = quote! { #(#field_names),* };

    // Separate required and optional fields for finish method
    let required_fields: Vec<&SimpleFieldEntry> =
        field_entries.iter().filter(|f| !f.is_optional).collect();
    let optional_fields: Vec<&SimpleFieldEntry> =
        field_entries.iter().filter(|f| f.is_optional).collect();

    // Generate required field checks - use different variable names to avoid conflicts
    let required_field_checks: Vec<TokenStream2> = required_fields
        .iter()
        .map(|entry| {
            let field_name = &entry.field_name;
            let checked_field_name = format_ident!("{}_checked", field_name);
            quote! {
                let #checked_field_name = #field_name.ok_or_else(|| {
                    xml::XmlError::InvalidXml(format!(
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
                // Optional fields pass through as-is
                quote! { #field_name }
            } else {
                // Required fields use the checked version
                let checked_field_name = format_ident!("{}_checked", field_name);
                quote! { #field_name: #checked_field_name }
            }
        })
        .collect();

    // Generate final field list for struct construction
    let final_field_list: Vec<&Ident> = field_entries.iter().map(|f| &f.field_name).collect();

    quote! {
        impl #impl_generics xml::parser::XmlVisitor<'a> for #visitor_name #ty_generics #where_clause {
            type Value = #struct_name #ty_generics;

            fn visit_children(
                &mut self,
                children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
            ) -> Result<(), xml::XmlError> {
                for child in children {
                    if !child.is_element() {
                        continue; // Skip non-element nodes like text/whitespace
                    }

                    let tag_name = child.tag_name().name();
                    let namespace = child.tag_name().namespace();

                    match tag_name {
                        #(#match_arms)*
                        _ => {
                            return Err(xml::XmlError::InvalidXml(format!(
                                "Unknown tag in {}: {tag_name}", stringify!(#struct_name)
                            )));
                        }
                    }
                }

                Ok(())
            }

            fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
                // Get the children and process them
                let children: Vec<_> = node.children().collect();

                self.visit_children(children.into_iter())?;
                Ok(())
            }

            fn finish(self) -> Result<Self::Value, xml::XmlError> {
                let Self { #field_list } = self;

                // Check required fields and extract values
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
        impl #impl_generics xml::parser::XmlDeserialize<'a> for #struct_name #ty_generics #where_clause {
            type Visitor = #visitor_name #ty_generics;

            fn visitor() -> Self::Visitor {
                #visitor_name::default()
            }

            fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
                xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
            }
        }
    }
}
