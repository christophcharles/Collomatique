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

    // Extract cache configuration from #[cached] or #[cached(Name)]
    let cache_config = extract_cache_attribute(&input.attrs, enum_name);

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

    // Generate cache struct if needed
    let cache_struct = if let Some(ref config) = cache_config {
        generate_cache_struct(config, &variant_info, &env_type, enum_name)
    } else {
        quote! {}
    };

    // Generate EvalObject implementation
    let eval_object_impl =
        generate_eval_object_impl(enum_name, &env_type, &variant_info, &cache_config);

    // Combine everything
    let expanded = quote! {
        #from_impls
        #cache_struct
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

// Cache configuration
struct CacheConfig {
    cache_name: syn::Ident,
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

fn extract_cache_attribute(attrs: &[Attribute], enum_name: &syn::Ident) -> Option<CacheConfig> {
    for attr in attrs {
        if attr.path().is_ident("cached") {
            // Check if it has a parameter: #[cached(Name)]
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(ident) = syn::parse2::<syn::Ident>(meta_list.tokens.clone()) {
                    return Some(CacheConfig { cache_name: ident });
                }
            } else {
                // No parameter: #[cached], auto-generate name
                let cache_name = syn::Ident::new(
                    &format!("{}Cache", enum_name),
                    proc_macro2::Span::call_site(),
                );
                return Some(CacheConfig { cache_name });
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
    variants: &[VariantInfo],
    cache_config: &Option<CacheConfig>,
) -> proc_macro2::TokenStream {
    // Determine cache type
    let cache_type = if let Some(config) = cache_config {
        let cache_name = &config.cache_name;
        quote! { #cache_name }
    } else {
        quote! { () }
    };

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

    // Generate the TypeId match arms for converting TypeId -> Object name
    let type_id_to_name_arms = variants.iter().map(|info| {
        let id_type = &info.id_type;
        let dsl_name = &info.dsl_type_name;

        quote! {
            id if id == ::std::any::TypeId::of::<#id_type>() => {
                Ok(#dsl_name.to_string())
            }
        }
    });

    // Generate field_access implementation (with or without caching)
    let field_access_arms = if cache_config.is_some() {
        generate_cached_field_access_arms(enum_name, env_type, variants)
    } else {
        generate_uncached_field_access_arms(enum_name, env_type, variants)
    };

    // Generate type_schemas implementation
    let type_schemas_entries = variants.iter().map(|info| {
        let dsl_name = &info.dsl_type_name;
        let id_type = &info.id_type;

        quote! {
            {
                let field_schema = <<#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::Object as ::collo_ml::ViewObject>::field_schema();
                let expr_schema = field_schema.into_iter()
                    .map(|(k, v)| (k, v.convert_to_expr_type::<#enum_name>().expect("Object type should be known")))
                    .collect();
                map.insert(#dsl_name.to_string(), expr_schema);
            }
        }
    });

    // Generate pretty_print implementation (with or without caching)
    let pretty_print_arms = if cache_config.is_some() {
        generate_cached_pretty_print_arms(enum_name, env_type, variants)
    } else {
        generate_uncached_pretty_print_arms(enum_name, env_type, variants)
    };

    quote! {
        impl ::collo_ml::EvalObject for #enum_name {
            type Env = #env_type;
            type Cache = #cache_type;

            fn objects_with_typ(env: &Self::Env, name: &str) -> ::std::collections::BTreeSet<Self> {
                match name {
                    #(#objects_with_typ_arms,)*
                    _ => ::std::collections::BTreeSet::new(),
                }
            }

            fn typ_name(&self, _env: &Self::Env) -> String {
                match self {
                    #(#typ_name_arms,)*
                }
            }

            fn type_id_to_name(type_id: ::std::any::TypeId) -> Result<String, ::collo_ml::traits::FieldConversionError> {
                match type_id {
                    #(#type_id_to_name_arms,)*
                    _ => Err(::collo_ml::traits::FieldConversionError::UnknownTypeId(type_id)),
                }
            }

            fn field_access(&self, env: &Self::Env, cache: &mut Self::Cache, field: &str) -> Option<::collo_ml::ExprValue<Self>> {
                match self {
                    #(#field_access_arms,)*
                }
            }

            fn type_schemas() -> ::std::collections::HashMap<String, ::std::collections::HashMap<String, ::collo_ml::ExprType>> {
                let mut map = ::std::collections::HashMap::new();
                #(#type_schemas_entries)*
                map
            }

            fn pretty_print(&self, env: &Self::Env, cache: &mut Self::Cache) -> Option<String> {
                match self {
                    #(#pretty_print_arms,)*
                }
            }
        }
    }
}

fn generate_cache_struct(
    config: &CacheConfig,
    variants: &[VariantInfo],
    env_type: &syn::Type,
    enum_name: &syn::Ident,
) -> proc_macro2::TokenStream {
    let cache_name = &config.cache_name;

    // Generate cache fields for each variant
    let cache_field_defs = variants.iter().map(|info| {
        let variant_name_lower = info.variant_name.to_string().to_lowercase();
        let field_name = syn::Ident::new(
            &format!("{}_cache", variant_name_lower),
            proc_macro2::Span::call_site(),
        );
        let id_type = &info.id_type;

        quote! {
            #field_name: ::std::collections::BTreeMap<
                #id_type,
                <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::Object
            >
        }
    });

    let cache_field_defaults = variants.iter().map(|info| {
        let variant_name_lower = info.variant_name.to_string().to_lowercase();
        let field_name = syn::Ident::new(
            &format!("{}_cache", variant_name_lower),
            proc_macro2::Span::call_site(),
        );

        quote! {
            #field_name: ::std::collections::BTreeMap::new()
        }
    });

    quote! {
        pub struct #cache_name {
            #(#cache_field_defs,)*
        }

        impl Default for #cache_name {
            fn default() -> Self {
                Self {
                    #(#cache_field_defaults,)*
                }
            }
        }
    }
}

fn generate_uncached_field_access_arms(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    variants: &[VariantInfo],
) -> Vec<proc_macro2::TokenStream> {
    variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;

        quote! {
            #enum_name::#variant_name(id) => {
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;
                Some(obj.get_field(field)?)
            }
        }
    }).collect()
}

fn generate_cached_field_access_arms(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    variants: &[VariantInfo],
) -> Vec<proc_macro2::TokenStream> {
    variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;
        let variant_name_lower = info.variant_name.to_string().to_lowercase();
        let cache_field = syn::Ident::new(
            &format!("{}_cache", variant_name_lower),
            proc_macro2::Span::call_site()
        );

        quote! {
            #enum_name::#variant_name(id) => {
                // Check cache first
                if let Some(cached_obj) = cache.#cache_field.get(id) {
                    return Some(cached_obj.get_field(field)?);
                }

                // Not in cache, build it
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;

                // Get field value before moving obj into cache
                let field_value = obj.get_field(field)?;

                // Store in cache (requires Clone)
                cache.#cache_field.insert(id.clone(), obj.clone());

                Some(field_value)
            }
        }
    }).collect()
}

fn generate_uncached_pretty_print_arms(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    variants: &[VariantInfo],
) -> Vec<proc_macro2::TokenStream> {
    variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;

        quote! {
            #enum_name::#variant_name(id) => {
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;
                obj.pretty_print()
            }
        }
    }).collect()
}

fn generate_cached_pretty_print_arms(
    enum_name: &syn::Ident,
    env_type: &syn::Type,
    variants: &[VariantInfo],
) -> Vec<proc_macro2::TokenStream> {
    variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let id_type = &info.id_type;
        let variant_name_lower = info.variant_name.to_string().to_lowercase();
        let cache_field = syn::Ident::new(
            &format!("{}_cache", variant_name_lower),
            proc_macro2::Span::call_site()
        );

        quote! {
            #enum_name::#variant_name(id) => {
                // Check cache first
                if let Some(cached_obj) = cache.#cache_field.get(id) {
                    return cached_obj.pretty_print();
                }

                // Not in cache, build it
                let obj = <#enum_name as ::collo_ml::ViewBuilder<#env_type, #id_type>>::build(env, id)?;

                // Get pretty print before moving obj into cache
                let result = obj.pretty_print();

                // Store in cache (requires Clone)
                cache.#cache_field.insert(id.clone(), obj.clone());

                result
            }
        }
    }).collect()
}
