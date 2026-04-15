use proc_macro::TokenStream;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericArgument, Lit, Meta, PathArguments, Type,
    Variant, parse_macro_input,
};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Extract enum name
    let enum_name = &input.ident;

    // Make sure it's an enum
    let variants = match &input.data {
        Data::Enum(data_enum) => &data_enum.variants,
        _ => panic!("EvalVar can only be derived for enums"),
    };

    // Extract env type from #[env(EnvType)] - now required
    let env_type = extract_env_attribute(&input.attrs)
        .expect("EvalVar derive requires #[env(EnvType)] attribute");

    // Extract value for fix_with if present
    let fix_with_expr = extract_fix_with_attribute(&input.attrs)
        .map(|expr| quote! { #expr })
        .unwrap_or(quote! { 0.0 });

    // Process each variant
    let mut variant_info = Vec::new();
    for variant in variants {
        let info = process_variant(variant, &fix_with_expr);
        variant_info.push(info);
    }

    // Generate the implementations
    let describe_var_impl = generate_describe_var_impl(enum_name, &variant_info, &env_type);
    let eval_var_impl = generate_eval_var_impl(enum_name, &variant_info);
    let try_from_impl = generate_try_from_impl(enum_name, &variant_info);

    // Combine everything
    let expanded = quote! {
        #describe_var_impl
        #eval_var_impl
        #try_from_impl
    };

    TokenStream::from(expanded)
}

enum FixType {
    FixWith(proc_macro2::TokenStream),
    DeferFix(proc_macro2::TokenStream),
}

// Helper struct to hold variant information
struct VariantInfo {
    variant_name: syn::Ident,    // e.g., "StudentInGroup"
    dsl_name: String,            // e.g., "SiG" or "StudentInGroup"
    fields: Vec<FieldInfo>,      // Field parameters
    var_type: Option<syn::Expr>, // Optional Variable type expression
    fix: FixType,
}

// Information about each field in a variant
struct FieldInfo {
    name: Option<syn::Ident>, // Field name if named struct
    ty: Type,                 // Field type
    range: Option<syn::Expr>, // Optional range for i32 fields
}

/// Check if a type is Option<T> and return the inner type T
fn unwrap_option_type(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty {
        let segment = type_path.path.segments.last()?;
        if segment.ident == "Option"
            && let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return Some(inner_ty);
        }
    }
    None
}

/// Check if type is Option<Option<T>> (nested options - not allowed)
fn is_nested_option(ty: &Type) -> bool {
    if let Some(inner) = unwrap_option_type(ty) {
        unwrap_option_type(inner).is_some()
    } else {
        false
    }
}

/// Get the core type, unwrapping Option if present
fn get_core_type(ty: &Type) -> &Type {
    unwrap_option_type(ty).unwrap_or(ty)
}

// Helper function to extract #[env(Type)]
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

// Helper function to extract #[fix_with(expr)]
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

// Helper function to extract #[defer_fix(expr)]
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

// Helper function to extract #[name("...")]
fn extract_name_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("name")
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(lit) = syn::parse2::<Lit>(meta_list.tokens.clone())
            && let Lit::Str(lit_str) = lit
        {
            return Some(lit_str.value());
        }
    }
    None
}

// Helper function to extract #[var(...)]
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

// Helper function to extract #[range(...)]
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

fn process_variant(variant: &Variant, fix_with_expr: &proc_macro2::TokenStream) -> VariantInfo {
    let variant_name = variant.ident.clone();

    // Extract DSL name from #[name("...")] or use variant name
    let dsl_name =
        extract_name_attribute(&variant.attrs).unwrap_or_else(|| variant_name.to_string());

    // Extract variable type from #[var(...)]
    let var_type = extract_var_attribute(&variant.attrs);

    // Extract default fix value for this variant if specified
    let variant_fix_with = extract_fix_with_attribute(&variant.attrs);
    let variant_defer_fix = extract_defer_fix_attribute(&variant.attrs);

    let fix = match (variant_defer_fix, variant_fix_with) {
        (Some(_), Some(_)) => {
            panic!("#[fix_with(...)] and #[defer_fix(...)] are mutually exclusive")
        }
        (Some(defer_fix), None) => FixType::DeferFix(quote! { #defer_fix }),
        (None, Some(fix_with)) => FixType::FixWith(quote! { #fix_with }),
        (None, None) => FixType::FixWith(fix_with_expr.clone()), // fall back to enum default
    };

    // Process fields
    let fields = match &variant.fields {
        Fields::Named(fields) => fields
            .named
            .iter()
            .map(|f| {
                let name = f.ident.clone();
                let ty = f.ty.clone();

                // Check for nested options
                if is_nested_option(&ty) {
                    panic!(
                        "Nested Option<Option<T>> is not supported in variant {} field {:?}",
                        variant_name, name
                    );
                }

                let range = extract_range_attribute(&f.attrs);
                FieldInfo { name, ty, range }
            })
            .collect(),
        Fields::Unnamed(fields) => fields
            .unnamed
            .iter()
            .enumerate()
            .map(|(idx, f)| {
                let ty = f.ty.clone();

                // Check for nested options
                if is_nested_option(&ty) {
                    panic!(
                        "Nested Option<Option<T>> is not supported in variant {} field {}",
                        variant_name, idx
                    );
                }

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
        dsl_name,
        fields,
        var_type,
        fix,
    }
}

fn generate_describe_var_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
    env_type: &syn::Type,
) -> proc_macro2::TokenStream {
    // Generate enumerate implementation (was vars)
    let vars_generation = generate_vars_impl(enum_name, variants);

    // Generate check_fix implementation (was fix)
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
        impl ::collo_ml::DescribeVar for #enum_name {
            type Env = #env_ty;

            fn enumerate(
                env: &#env_ty
            ) -> ::std::collections::HashMap<Self, ::collomatique_ilp::Variable> {
                #vars_generation
            }

            fn check_fix(&self, env: &#env_ty) -> Option<f64> {
                match self {
                    #(#fix_arms,)*
                }
            }
        }
    }
}

fn generate_eval_var_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    let field_schema_entries = variants.iter().map(|info| {
        let dsl_name = &info.dsl_name;
        let field_types = info
            .fields
            .iter()
            .map(|field| generate_field_type_expr(&field.ty));

        quote! {
            schema.insert(
                #dsl_name.to_string(),
                vec![#(#field_types),*]
            );
        }
    });

    quote! {
        impl ::collo_ml::EvalVar for #enum_name {
            fn field_schema() -> ::std::collections::HashMap<String, Vec<::collo_ml::ExprType>> {
                let mut schema = ::std::collections::HashMap::new();
                #(#field_schema_entries)*
                schema
            }
        }
    }
}

fn generate_field_type_expr(ty: &Type) -> proc_macro2::TokenStream {
    // Check if it's Option<T>
    if let Some(inner_ty) = unwrap_option_type(ty) {
        // For Option<T>, we need to generate: ExprType::sum(vec![SimpleType::None, <inner type>])
        let inner_type_expr = generate_field_type_expr_core(inner_ty);

        return quote! {
            ::collo_ml::ExprType::sum(vec![
                ::collo_ml::SimpleType::None,
                #inner_type_expr
            ]).expect("Should have at least one variant")
        };
    }

    // Not an Option, generate as before
    generate_field_type_expr_core(ty)
}

fn generate_field_type_expr_core(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => quote! { ::collo_ml::SimpleType::Int.into() },
                "bool" => quote! { ::collo_ml::SimpleType::Bool.into() },
                "Vec" => panic!("List are not supported as variable parameters: {:?}", ty),
                "Option" => panic!("Should not reach here - Option should be handled by caller"),
                _ => {
                    panic!(
                        "Unsupported field type '{}' in EvalVar derive. Supported types: i32, bool, Option<T>",
                        type_name
                    )
                }
            }
        }
        _ => panic!("Unsupported field type: {:?}", ty),
    }
}

fn generate_vars_impl(
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

        // Generate nested loops for each field
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
        // Unit variant
        return quote! {
            vars.insert(#enum_name::#variant_name, #var_type);
        };
    }

    // Generate loop for each field, from outermost to innermost
    let mut loops = Vec::new();
    let mut var_names = Vec::new();

    for (idx, field) in fields.iter().enumerate() {
        let var_name = syn::Ident::new(&format!("v{}", idx), proc_macro2::Span::call_site());

        let binding = if let Some(field_name) = &field.name {
            quote! {
                let #field_name = &#var_name;
                let _ = #field_name; // To avoid unused warnings
            }
        } else {
            quote! {}
        };

        let loop_code = generate_field_loop(&field.ty, &var_name, &field.range);
        loops.push((loop_code, binding));
        var_names.push(var_name);
    }

    // Build the variant construction
    let variant_construction = if fields.iter().all(|f| f.name.is_some()) {
        // Named fields
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
        // Unnamed fields
        quote! {
            #enum_name::#variant_name(#(#var_names),*)
        }
    };

    // Nest the loops from innermost to outermost
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
    let iterator_expr = generate_field_iterator(ty, range);

    // Check if it's Option<T>
    if let Some(_inner_ty) = unwrap_option_type(ty) {
        // For Option, chain None with Some(inner values)
        quote! {
            for #var_name in ::std::iter::once(None).chain(
                (#iterator_expr).map(Some)
            )
        }
    } else {
        // For non-Option, just iterate normally
        quote! {
            for #var_name in #iterator_expr
        }
    }
}

fn generate_field_iterator(ty: &Type, range: &Option<syn::Expr>) -> proc_macro2::TokenStream {
    // Get the core type (unwrap Option if present)
    let core_ty = get_core_type(ty);

    match core_ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => {
                    if let Some(range_expr) = range {
                        quote! { #range_expr }
                    } else {
                        panic!("i32 fields must have a #[range(...)] attribute");
                    }
                }
                "bool" => {
                    if range.is_some() {
                        panic!("#[range(...)] attribute is not supported for bool type");
                    }
                    quote! { [false, true] }
                }
                "Option" => {
                    panic!("Should not reach here - Option should be handled by caller")
                }
                _ => {
                    panic!(
                        "Unsupported field type '{}' in EvalVar derive. Supported types: i32, bool, Option<T>",
                        type_name
                    )
                }
            }
        }
        _ => panic!("Unsupported field type"),
    }
}

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
                    // cannot be out of range with unit variant. No test needed
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
                bindings.push(quote! { let _ = #field_name; }); // To avoid unused field warning
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
                let var_name = match &field.name {
                    Some(field_name) => field_name.clone(),
                    None => syn::Ident::new(&format!("v{}", idx), proc_macro2::Span::call_site()),
                };

                // Generate range check for i32 fields (including Option<i32>)
                let core_ty = get_core_type(&field.ty);
                let is_option = unwrap_option_type(&field.ty).is_some();

                if let Type::Path(type_path) = core_ty {
                    let segment = type_path.path.segments.last().unwrap();
                    let type_name = segment.ident.to_string();

                    if type_name == "i32"
                        && let Some(range_expr) = &field.range
                    {
                        let check = if is_option {
                            // For Option<i32>, check if Some and in range
                            quote! {
                                if let Some(val) = #var_name {
                                    if !(#range_expr).contains(val) {
                                        return Some(#fix_with);
                                    }
                                }
                            }
                        } else {
                            // For i32, check if in range
                            quote! {
                                if !(#range_expr).contains(#var_name) {
                                    return Some(#fix_with);
                                }
                            }
                        };
                        checks.push(check);
                    }
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

fn generate_try_from_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
) -> proc_macro2::TokenStream {
    let match_arms = variants.iter().map(|info| {
        let dsl_name = &info.dsl_name;
        let expected_count = info.fields.len();

        // Generate parameter extraction
        let (param_extractions, variant_construction) = generate_param_conversions(enum_name, info);

        quote! {
            #dsl_name => {
                if value.params.len() != #expected_count {
                    return Err(::collo_ml::traits::VarConversionError::WrongParameterCount {
                        name: #dsl_name.into(),
                        expected: #expected_count,
                        found: value.params.len(),
                    });
                }
                #(#param_extractions)*
                Ok(#variant_construction)
            }
        }
    });

    quote! {
        impl<__D: ::collo_ml::DatabaseConnection> TryFrom<&::collo_ml::eval::ExternVar<__D>> for #enum_name {
            type Error = ::collo_ml::traits::VarConversionError;

            fn try_from(value: &::collo_ml::eval::ExternVar<__D>) -> Result<Self, Self::Error> {
                match value.name.as_str() {
                    #(#match_arms,)*
                    _ => Err(::collo_ml::traits::VarConversionError::Unknown(value.name.clone())),
                }
            }
        }
    }
}

fn generate_param_conversions(
    enum_name: &syn::Ident,
    info: &VariantInfo,
) -> (Vec<proc_macro2::TokenStream>, proc_macro2::TokenStream) {
    let mut extractions = Vec::new();
    let mut field_values = Vec::new();

    for (idx, field) in info.fields.iter().enumerate() {
        let param_name = syn::Ident::new(&format!("param{}", idx), proc_macro2::Span::call_site());
        let dsl_name = &info.dsl_name;

        let extraction = generate_param_extraction(&field.ty, idx, &param_name, dsl_name);
        extractions.push(extraction);

        if let Some(field_name) = &field.name {
            field_values.push(quote! { #field_name: #param_name });
        } else {
            field_values.push(quote! { #param_name });
        }
    }

    let variant_name = &info.variant_name;
    let construction = if info.fields.iter().all(|f| f.name.is_some()) {
        quote! { #enum_name::#variant_name { #(#field_values),* } }
    } else if info.fields.is_empty() {
        quote! { #enum_name::#variant_name }
    } else {
        quote! { #enum_name::#variant_name(#(#field_values),*) }
    };

    (extractions, construction)
}

fn generate_param_extraction(
    ty: &Type,
    idx: usize,
    param_name: &syn::Ident,
    dsl_name: &str,
) -> proc_macro2::TokenStream {
    let is_option = unwrap_option_type(ty).is_some();
    let core_ty = get_core_type(ty);

    let none_arm = if is_option {
        quote! { ::collo_ml::ExprValue::None => None, }
    } else {
        quote! {}
    };

    match core_ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => {
                    let opt_output = if is_option {
                        quote! { Some(*i) }
                    } else {
                        quote! { *i }
                    };
                    quote! {
                        let #param_name = match &*value.params[#idx] {
                            #none_arm
                            ::collo_ml::ExprValue::Int(i) => #opt_output,
                            _ => {
                                return Err(::collo_ml::traits::VarConversionError::WrongParameterType {
                                    name: #dsl_name.into(),
                                    param: #idx,
                                    expected: ::collo_ml::SimpleType::Int.into(),
                                })
                            }
                        };
                    }
                }
                "bool" => {
                    let opt_output = if is_option {
                        quote! { Some(*b) }
                    } else {
                        quote! { *b }
                    };
                    quote! {
                        let #param_name = match &*value.params[#idx] {
                            #none_arm
                            ::collo_ml::ExprValue::Bool(b) => #opt_output,
                            _ => {
                                return Err(::collo_ml::traits::VarConversionError::WrongParameterType {
                                    name: #dsl_name.into(),
                                    param: #idx,
                                    expected: ::collo_ml::SimpleType::Bool.into(),
                                })
                            }
                        };
                    }
                }
                "Option" => {
                    panic!("Should not reach here - Option should be handled by caller")
                }
                _ => {
                    panic!(
                        "Unsupported parameter type '{}' in EvalVar derive. Supported types: i32, bool, Option<T>",
                        type_name
                    )
                }
            }
        }
        _ => panic!("Unsupported parameter type"),
    }
}
