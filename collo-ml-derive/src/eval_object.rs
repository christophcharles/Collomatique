use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Meta, Variant, parse_macro_input};

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

    // Generate EvalObject implementation
    let eval_object_impl = generate_eval_object_impl(enum_name, &env_type, &variant_info);

    // Combine everything
    let expanded = quote! {
        #from_impls
        #eval_object_impl
    };

    TokenStream::from(expanded)
}

// Helper struct to hold variant information
struct VariantInfo {
    variant_name: syn::Ident, // e.g., "Student"
    id_type: syn::Type,       // e.g., StudentId
    _dsl_type_name: String,   // e.g., "Student" or custom from #[name("...")]
}

fn extract_env_type(attrs: &[Attribute]) -> Option<syn::Type> {
    for attr in attrs {
        if attr.path().is_ident("env")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(ty) = syn::parse2::<syn::Type>(meta_list.tokens.clone())
        {
            return Some(ty);
        }
    }
    None
}

fn extract_name_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("name")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(lit) = syn::parse2::<syn::Lit>(meta_list.tokens.clone())
            && let syn::Lit::Str(lit_str) = lit
        {
            return Some(lit_str.value());
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
        _dsl_type_name: dsl_type_name,
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

            impl TryFrom<#enum_name> for #id_type {
                type Error = ::collo_ml::traits::TypeConversionError;

                fn try_from(value: #enum_name) -> Result<Self, Self::Error> {
                    use ::collo_ml::traits::TypeConversionError;
                    match value {
                        #enum_name::#variant_name(id) => Ok(id),
                        _ => Err(TypeConversionError::BadType),
                    }
                }
            }
        }
    });

    quote! {
        #(#impls)*
    }
}

fn generate_eval_object_impl(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    _variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    quote! {
        impl ::collo_ml::EvalObject for #enum_name {
            type Env = #env_type;
        }
    }
}
