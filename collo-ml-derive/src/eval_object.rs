use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Attribute, Data, DeriveInput, Fields, Meta, Variant};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract enum name
    let enum_name = &input.ident;

    // Extract the Env type from #[env(MyEnv)]
    let env_type =
        extract_env_type(&input.attrs).expect("EvalObject requires #[env(YourEnvType)] attribute");

    // Make sure it's an enum
    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("EvalObject can only be derived for enums"),
    };

    // Process each variant
    let mut variant_info = Vec::new();
    for variant in variants {
        let info = process_variant(variant);
        variant_info.push(info);
    }

    // Generate From<IdType> implementations
    let from_impls = generate_from_impls(enum_name, &variant_info);

    // Generate helper functions
    let helper_functions = generate_helper_functions(enum_name, &variant_info);

    // Generate EvalObject implementation
    let eval_object_impl = generate_eval_object_impl(enum_name, &env_type, &variant_info);

    // Combine everything
    let expanded = quote! {
        #from_impls
        #helper_functions
        #eval_object_impl
    };

    TokenStream::from(expanded)
}

// Helper struct to hold variant information
struct VariantInfo {
    variant_name: syn::Ident, // e.g., "Student"
    id_type: syn::Type,       // e.g., StudentId
    dsl_type_name: String,    // e.g., "Student" or custom from #[name("...")]
}

fn extract_env_type(attrs: &[Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident("env") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(ty) = syn::parse2::<syn::Type>(meta_list.tokens.clone()) {
                    return Some(ty);
                }
            }
        }
    }
    None
}

fn extract_name_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("name") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(lit) = syn::parse2::<syn::Lit>(meta_list.tokens.clone()) {
                    if let syn::Lit::Str(lit_str) = lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

fn process_variant(variant: &Variant) -> VariantInfo {
    let variant_name = variant.ident.clone();

    // Extract the ID type from the variant (assumes single unnamed field)
    let id_type = match &variant.fields {
        Fields::Unnamed(fields) => {
            if fields.unnamed.len() != 1 {
                panic!("Each enum variant must have exactly one field");
            }
            fields.unnamed.first().unwrap().ty.clone()
        }
        _ => panic!("Enum variants must have a single unnamed field, e.g., Student(StudentId)"),
    };

    // Check for #[name("...")] attribute, otherwise use variant name
    let dsl_type_name =
        extract_name_attribute(&variant.attrs).unwrap_or_else(|| variant_name.to_string());

    VariantInfo {
        variant_name,
        id_type,
        dsl_type_name,
    }
}

fn generate_from_impls(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    let impls = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;

        quote! {
            impl From<#id_type> for #enum_name {
                fn from(id: #id_type) -> Self {
                    #enum_name::#variant_name(id)
                }
            }
        }
    });

    quote! {
        #(#impls)*
    }
}

fn generate_helper_functions(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    // Generate the TypeId match arms for converting FieldType::Object -> ExprType::Object
    let type_id_to_name_arms = variants.iter().map(|info| {
        let id_type = &info.id_type;
        let dsl_name = &info.dsl_type_name;

        quote! {
            id if id == std::any::TypeId::of::<#id_type>() => {
                ::collo_ml::ExprType::Object(#dsl_name.to_string())
            }
        }
    });

    quote! {
        impl #enum_name {
            #[doc(hidden)]
            fn __collo_ml_convert_field_type(field_type: ::collo_ml::traits::FieldType) -> ::collo_ml::ExprType {
                match field_type {
                    ::collo_ml::traits::FieldType::Int => ::collo_ml::ExprType::Int,
                    ::collo_ml::traits::FieldType::Bool => ::collo_ml::ExprType::Bool,
                    ::collo_ml::traits::FieldType::Object(type_id) => {
                        match type_id {
                            #(#type_id_to_name_arms,)*
                            _ => panic!("Unknown object type: {:?}", type_id),
                        }
                    }
                    ::collo_ml::traits::FieldType::List(inner) => {
                        ::collo_ml::ExprType::List(Box::new(Self::__collo_ml_convert_field_type(*inner)))
                    }
                }
            }

            #[doc(hidden)]
            fn __collo_ml_convert_field_value(value: ::collo_ml::traits::FieldValue<Self>) -> ::collo_ml::ExprValue<Self> {
                match value {
                    ::collo_ml::traits::FieldValue::Int(i) => ::collo_ml::ExprValue::Int(i),
                    ::collo_ml::traits::FieldValue::Bool(b) => ::collo_ml::ExprValue::Bool(b),
                    ::collo_ml::traits::FieldValue::Object(obj) => ::collo_ml::ExprValue::Object(obj),
                    ::collo_ml::traits::FieldValue::List(field_type, items) => {
                        let expr_type = Self::__collo_ml_convert_field_type(field_type);
                        let converted_items = items.into_iter()
                            .map(Self::__collo_ml_convert_field_value)
                            .collect();
                        ::collo_ml::ExprValue::List(expr_type, converted_items)
                    }
                }
            }
        }
    }
}

fn generate_eval_object_impl(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    // Generate objects_with_typ implementation
    let objects_with_typ_arms = variants.iter().map(|info| {
        let dsl_name = &info.dsl_type_name;
        let id_type = &info.id_type;
        let variant_name = &info.variant_name;

        quote! {
            #dsl_name => {
                <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::enumerate(env)
                    .into_iter()
                    .map(#enum_name::#variant_name)
                    .collect()
            }
        }
    });

    // Generate typ_name implementation
    let typ_name_arms = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let dsl_name = &info.dsl_type_name;

        quote! {
            #enum_name::#variant_name(_) => #dsl_name.to_string()
        }
    });

    // Generate field_access implementation
    let field_access_arms = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;

        quote! {
            #enum_name::#variant_name(id) => {
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;
                let field_value = obj.get_field(field)?;
                Some(Self::__collo_ml_convert_field_value(field_value))
            }
        }
    });

    // Generate type_schemas implementation
    let type_schemas_entries = variants.iter().map(|info| {
        let dsl_name = &info.dsl_type_name;
        let id_type = &info.id_type;

        quote! {
            {
                let field_schema = <<#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::Object as ::collo_ml::ViewObject>::field_schema();
                let expr_schema = field_schema.into_iter()
                    .map(|(k, v)| (k, #enum_name::__collo_ml_convert_field_type(v)))
                    .collect();
                map.insert(#dsl_name.to_string(), expr_schema);
            }
        }
    });

    // Generate pretty_print implementation
    let pretty_print_arms = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;

        quote! {
            #enum_name::#variant_name(id) => {
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;
                obj.pretty_print()
            }
        }
    });

    quote! {
        impl ::collo_ml::EvalObject for #enum_name {
            type Env = #env_type;

            fn objects_with_typ(env: &Self::Env, name: &str) -> std::collections::BTreeSet<Self> {
                match name {
                    #(#objects_with_typ_arms,)*
                    _ => std::collections::BTreeSet::new(),
                }
            }

            fn typ_name(&self, _env: &Self::Env) -> String {
                match self {
                    #(#typ_name_arms,)*
                }
            }

            fn field_access(&self, env: &Self::Env, field: &str) -> Option<::collo_ml::ExprValue<Self>> {
                match self {
                    #(#field_access_arms,)*
                }
            }

            fn type_schemas() -> std::collections::HashMap<String, std::collections::HashMap<String, ::collo_ml::ExprType>> {
                let mut map = std::collections::HashMap::new();
                #(#type_schemas_entries)*
                map
            }

            fn pretty_print(&self, env: &Self::Env) -> Option<String> {
                match self {
                    #(#pretty_print_arms,)*
                }
            }
        }
    }
}
