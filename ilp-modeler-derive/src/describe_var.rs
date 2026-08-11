use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Expr, Fields, Meta, Type, Variant, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let enum_name = &input.ident;

    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("DescribeVar can only be derived for enums"),
    };

    let env_type = extract_env_attribute(&input.attrs)
        .expect("DescribeVar derive requires #[env(EnvType)] attribute");

    let fix_with_expr = extract_fix_with_attribute(&input.attrs)
        .map(|expr| quote! { #expr })
        .unwrap_or(quote! { 0.0 });

    let mut variant_info = Vec::new();
    for variant in variants {
        let info = process_variant(variant, &fix_with_expr);
        variant_info.push(info);
    }

    let describe_var_impl = generate_describe_var_impl(enum_name, &variant_info, &env_type);

    TokenStream::from(describe_var_impl)
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

enum FixType {
    FixWith(proc_macro2::TokenStream),
    DeferFix(proc_macro2::TokenStream),
}

struct VariantInfo {
    variant_name: syn::Ident,
    fields: Vec<FieldInfo>,
    var_type: Option<syn::Expr>,
    fix: FixType,
}

struct FieldInfo {
    name: Option<syn::Ident>,
    ty: Type,
    range: Option<syn::Expr>,
}

// ---------------------------------------------------------------------------
// Attribute extraction
// ---------------------------------------------------------------------------

fn extract_env_attribute(attrs: &[Attribute]) -> Option<syn::Type> {
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

fn extract_fix_with_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("fix_with")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr);
        }
    }
    None
}

fn extract_defer_fix_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("defer_fix")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr);
        }
    }
    None
}

fn extract_var_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("var")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr);
        }
    }
    None
}

fn extract_range_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("range")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Variant processing
// ---------------------------------------------------------------------------

fn process_variant(variant: &Variant, fix_with_expr: &proc_macro2::TokenStream) -> VariantInfo {
    let variant_name = variant.ident.clone();

    let var_type = extract_var_attribute(&variant.attrs);

    let variant_fix_with = extract_fix_with_attribute(&variant.attrs);
    let variant_defer_fix = extract_defer_fix_attribute(&variant.attrs);

    let fix = match (variant_defer_fix, variant_fix_with) {
        (Some(_), Some(_)) => {
            panic!("#[fix_with(...)] and #[defer_fix(...)] are mutually exclusive")
        }
        (Some(defer_fix), None) => FixType::DeferFix(quote! { #defer_fix }),
        (None, Some(fix_with)) => FixType::FixWith(quote! { #fix_with }),
        (None, None) => FixType::FixWith(fix_with_expr.clone()),
    };

    let fields = match &variant.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| {
                let name = f.ident.clone();
                let ty = f.ty.clone();
                let range = extract_range_attribute(&f.attrs);
                FieldInfo { name, ty, range }
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .map(|f| {
                let ty = f.ty.clone();
                let range = extract_range_attribute(&f.attrs);
                FieldInfo {
                    name: None,
                    ty,
                    range,
                }
            })
            .collect(),
        Fields::Unit => Vec::new(),
    };

    VariantInfo {
        variant_name,
        fields,
        var_type,
        fix,
    }
}

// ---------------------------------------------------------------------------
// Code generation: impl DescribeVar
// ---------------------------------------------------------------------------

fn generate_describe_var_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
    env_type: &syn::Type,
) -> proc_macro2::TokenStream {
    let enumerate_body = generate_enumerate_impl(enum_name, variants);

    let fix_arms = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let (pattern, checks_and_output) = generate_fix_pattern_and_checks_and_output(info);

        quote! {
            #enum_name::#variant_name #pattern => {
                #checks_and_output
            }
        }
    });

    let env_ty = env_type;

    quote! {
        impl ::collomatique_ilp_modeler::DescribeVar for #enum_name {
            type Env = #env_ty;

            fn enumerate(
                env: &#env_ty
            ) -> ::std::collections::HashMap<Self, ::collomatique_ilp::Variable> {
                #enumerate_body
            }

            fn check_fix(&self, env: &#env_ty) -> Option<f64> {
                match self {
                    #(#fix_arms,)*
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Code generation: enumerate()
// ---------------------------------------------------------------------------

fn generate_enumerate_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    let variant_iterations = variants.iter().map(|info| {
        let variant_name = &info.variant_name;
        let var_type = info
            .var_type
            .as_ref()
            .map(|expr| quote! { #expr })
            .unwrap_or_else(|| {
                quote! { Variable::binary() }
            });

        generate_field_iterations(
            enum_name,
            variant_name,
            &info.fields,
            &var_type,
            matches!(info.fix, FixType::DeferFix(_)),
        )
    });

    quote! {
        use ::collomatique_ilp::Variable;
        let mut vars = ::std::collections::HashMap::new();
        #(#variant_iterations)*
        vars
    }
}

fn generate_field_iterations(
    enum_name: &syn::Ident,
    variant_name: &syn::Ident,
    fields: &[FieldInfo],
    var_type: &proc_macro2::TokenStream,
    defered_fix: bool,
) -> proc_macro2::TokenStream {
    if fields.is_empty() {
        return quote! {
            vars.insert(#enum_name::#variant_name, #var_type);
        };
    }

    let mut loops = Vec::new();
    let mut var_names = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let var_name = syn::Ident::new(&format!("v{}", idx), proc_macro2::Span::call_site());

        let binding = if let Some(field_name) = &field.name {
            quote! {
                let #field_name = &#var_name;
                let _ = #field_name;
            }
        } else {
            quote! {}
        };

        let loop_code = generate_field_loop(&field.ty, &var_name, &field.range);
        loops.push((loop_code, binding));
        var_names.push(var_name);
    }

    let variant_construction = if fields.iter().all(|f| f.name.is_some()) {
        let field_assignments = fields
            .iter()
            .zip(var_names.iter())
            .map(|(field, var_name)| {
                let field_name = field.name.as_ref().unwrap();
                quote! { #field_name: #var_name }
            });
        quote! {
            #enum_name::#variant_name { #(#field_assignments),* }
        }
    } else {
        quote! {
            #enum_name::#variant_name(#(#var_names),*)
        }
    };

    let mut inner_code = if defered_fix {
        quote! {
            let new_var = #variant_construction;
            if new_var.check_fix(env).is_some() {
                continue;
            }
            vars.insert(new_var, #var_type);
        }
    } else {
        quote! {
            vars.insert(#variant_construction, #var_type);
        }
    };

    for (loop_code, binding) in loops.into_iter().rev() {
        inner_code = quote! {
            #loop_code {
                #binding
                #inner_code
            }
        };
    }

    inner_code
}

fn generate_field_loop(
    ty: &Type,
    var_name: &syn::Ident,
    range: &Option<syn::Expr>,
) -> proc_macro2::TokenStream {
    let iterator_expr = if let Some(range_expr) = range {
        quote! {
            <#ty as ::collomatique_ilp_modeler::EnumerateFrom<_>>::enumerate_from(#range_expr)
        }
    } else {
        quote! {
            <#ty as ::collomatique_ilp_modeler::EnumerateAll>::enumerate_all()
        }
    };

    quote! {
        for #var_name in #iterator_expr
    }
}

// ---------------------------------------------------------------------------
// Code generation: check_fix()
// ---------------------------------------------------------------------------

fn generate_fix_pattern_and_checks_and_output(
    info: &VariantInfo,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream) {
    let fix = &info.fix;

    if info.fields.is_empty() {
        return (
            quote! {},
            match fix {
                FixType::DeferFix(defer_fix) => quote! {
                    #defer_fix
                },
                FixType::FixWith(_) => {
                    quote! { None }
                }
            },
        );
    }

    let mut field_patterns = Vec::new();
    let mut bindings = Vec::new();

    for (idx, field) in info.fields.iter().enumerate() {
        let var_name = match &field.name {
            Some(field_name) => {
                bindings.push(quote! { let _ = #field_name; });
                field_name.clone()
            }
            None => syn::Ident::new(&format!("v{}", idx), proc_macro2::Span::call_site()),
        };
        field_patterns.push(quote! { #var_name });
    }

    let pattern = if info.fields.iter().all(|f| f.name.is_some()) {
        quote! { { #(#field_patterns),* } }
    } else {
        quote! { ( #(#field_patterns),* ) }
    };

    let checks_and_output_code = match fix {
        FixType::DeferFix(defer_fix) => {
            quote! {
                #(#bindings)*
                #defer_fix
            }
        }
        FixType::FixWith(fix_with) => {
            let mut checks = Vec::new();

            for (idx, field) in info.fields.iter().enumerate() {
                if let Some(range_expr) = &field.range {
                    let var_name = match &field.name {
                        Some(field_name) => field_name.clone(),
                        None => {
                            syn::Ident::new(&format!("v{}", idx), proc_macro2::Span::call_site())
                        }
                    };

                    let ty = &field.ty;
                    let check = quote! {
                        if !<#ty as ::collomatique_ilp_modeler::EnumerateFrom<_>>::enumerate_from(#range_expr)
                            .contains(#var_name)
                        {
                            return Some(#fix_with);
                        }
                    };
                    checks.push(check);
                }
            }

            quote! {
                #(#bindings)*
                #(#checks)*
                None
            }
        }
    };

    (pattern, checks_and_output_code)
}
