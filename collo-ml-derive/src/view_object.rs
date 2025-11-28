use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Fields, GenericArgument, Lit, Meta,
    PathArguments, Type,
};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract struct name
    let name = &input.ident;

    // Extract the EvalObject type from #[eval_object(EvalObject)]
    let eval_object_type = extract_eval_object_type(&input.attrs)
        .expect("ViewObject requires #[eval_object(YourEvalObjectType)] attribute");

    // Make sure it's a struct with named fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("ViewObject only works on structs with named fields"),
        },
        _ => panic!("ViewObject can only be derived for structs"),
    };

    // Extract pretty print format if present
    let pretty_print_impl = extract_pretty_format(&input.attrs, fields);

    // Process each field
    let mut field_schema_entries = Vec::new();
    let mut field_access_arms = Vec::new();

    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        // Check if field is hidden
        if has_hidden_attribute(&field.attrs) {
            continue; // Skip hidden fields
        }

        // Generate schema entry
        let expr_type = type_to_field_type(field_type);
        field_schema_entries.push(quote! {
            schema.insert(#field_name_str.to_string(), #expr_type);
        });

        // Generate field access arm
        let field_value = generate_field_value(
            &quote! {
                self.#field_name
            },
            field_type,
        );
        field_access_arms.push(quote! {
            #field_name_str => Some(#field_value),
        });
    }

    // Generate the implementation
    let expanded = quote! {
        impl ::collo_ml::ViewObject for #name {
            type EvalObject = #eval_object_type;

            fn field_schema() -> std::collections::HashMap<String, ::collo_ml::traits::FieldType> {
                let mut schema = std::collections::HashMap::new();
                #(#field_schema_entries)*
                schema
            }

            fn get_field(&self, field: &str) -> Option<::collo_ml::traits::FieldValue<Self::EvalObject>> {
                match field {
                    #(#field_access_arms)*
                    _ => None,
                }
            }

            #pretty_print_impl
        }
    };

    TokenStream::from(expanded)
}

// Helper function to extract #[eval_object(Type)]
fn extract_eval_object_type(attrs: &[Attribute]) -> Option<syn::Ident> {
    for attr in attrs {
        if attr.path().is_ident("eval_object") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the tokens inside the parentheses
                if let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone()) {
                    return Some(ident);
                }
            }
        }
    }
    None
}

// Helper to check if field has #[hidden]
fn has_hidden_attribute(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| attr.path().is_ident("hidden"))
}

// Helper to extract #[pretty("format string")]
fn extract_pretty_format(
    attrs: &[Attribute],
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    for attr in attrs {
        if attr.path().is_ident("pretty") {
            if let Meta::List(meta_list) = &attr.meta {
                // Parse the string literal
                if let Ok(lit) = syn::parse2::<Lit>(meta_list.tokens.clone()) {
                    if let Lit::Str(lit_str) = lit {
                        let format_str = lit_str.value();
                        // Generate format! call with field accesses
                        return generate_pretty_print_from_format(&format_str, fields);
                    }
                }
            }
        }
    }

    // Default implementation
    quote! {
        fn pretty_print(&self) -> Option<String> {
            None
        }
    }
}

fn type_to_field_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(type_path) => {
            // Get the last segment of the path (e.g., "i32" from "std::i32")
            let segment = type_path.path.segments.last().unwrap();
            let type_name = &segment.ident;
            let type_name_str = type_name.to_string();

            match type_name_str.as_str() {
                "i32" => quote! { ::collo_ml::traits::FieldType::Int },
                "bool" => quote! { ::collo_ml::traits::FieldType::Bool },
                "BTreeSet" => {
                    // Extract the inner type from BTreeSet<T>
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_expr_type = type_to_field_type(inner_ty);
                            return quote! {
                                ::collo_ml::traits::FieldType::List(Box::new(#inner_expr_type))
                            };
                        }
                    }
                    panic!("BTreeSet must have a type parameter");
                }
                _ => {
                    // Assume this is an object
                    quote! { ::collo_ml::traits::FieldType::Object(std::any::TypeId::of::<#type_name>()) }
                }
            }
        }
        _ => panic!("Unsupported type: {:?}", ty),
    }
}

fn generate_field_value(
    field_name: &proc_macro2::TokenStream,
    field_type: &Type,
) -> proc_macro2::TokenStream {
    match field_type {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => quote! {
                    ::collo_ml::traits::FieldValue::Int(#field_name.clone()),
                },
                "bool" => quote! {
                    ::collo_ml::traits::FieldValue::Bool(#field_name.clone()),
                },
                "BTreeSet" => {
                    // Need to convert collection elements
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            let field_type = type_to_field_type(inner_ty);
                            let inner = generate_field_value(
                                &quote! {
                                    x
                                },
                                inner_ty,
                            );
                            return quote! {
                                ::collo_ml::traits::FieldValue::List(
                                    #field_type,
                                    #field_name.iter().map(|x| #inner).collect(),
                                )
                            };
                        }
                    }
                    panic!("BTreeSet must have type parameter");
                }
                _ => {
                    // It's an object ID - convert using Into
                    quote! {
                        ::collo_ml::traits::FieldValue::Object(#field_name.clone().into()),
                    }
                }
            }
        }
        _ => panic!("Unsupported field type"),
    }
}

fn generate_pretty_print_from_format(
    format_str: &str,
    fields: &syn::punctuated::Punctuated<syn::Field, syn::token::Comma>,
) -> proc_macro2::TokenStream {
    // Extract field names that can be referenced
    let field_names: Vec<String> = fields
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    // Parse the format string to find {field_name} placeholders
    // We'll use a simple regex-like approach
    let mut format_args = std::collections::BTreeMap::new();
    let mut current_pos = 0;

    // Simple parser for {field_name}
    while let Some(start) = format_str[current_pos..].find('{') {
        let start = current_pos + start;
        if let Some(end) = format_str[start..].find(|c| c == '}' || c == ':') {
            let end = start + end;
            let field_name = &format_str[start + 1..end];

            // Validate that this field exists
            if !field_names.contains(&field_name.to_string()) {
                panic!("Format string references unknown field: {}", field_name);
            }

            let field_ident = syn::Ident::new(field_name, proc_macro2::Span::call_site());
            format_args.insert(
                field_name.to_string(),
                quote! { let #field_ident = &self.#field_ident; },
            );

            current_pos = end + 1;
        } else {
            break;
        }
    }

    // Generate the format! call
    if format_args.is_empty() {
        // No placeholders, just return the string as-is
        quote! {
            fn pretty_print(&self) -> Option<String> {
                Some(#format_str.to_string())
            }
        }
    } else {
        let format_args = format_args.into_iter().map(|x| x.1);
        quote! {
            fn pretty_print(&self) -> Option<String> {
                #(#format_args)*
                Some(format!(#format_str))
            }
        }
    }
}
