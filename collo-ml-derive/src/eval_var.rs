use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, Lit, Meta, Type, Variant,
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

    // Extract optional env type from #[env(EnvType)]
    let env_type = extract_env_attribute(&input.attrs);

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
    let eval_var_impl = generate_eval_var_impl(enum_name, &variant_info, env_type.as_ref());
    let try_from_impl = generate_try_from_impl(enum_name, &variant_info);

    // Combine everything
    let expanded = quote! {
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

// Helper function to extract #[env(Type)]
fn extract_env_attribute(attrs: &[Attribute]) -> Option<syn::Type> {
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

// Helper function to extract #[fix_with(expr)]
fn extract_fix_with_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("fix_with") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone()) {
                    return Some(expr);
                }
            }
        }
    }
    None
}

// Helper function to extract #[defer_fix(expr)]
fn extract_defer_fix_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("defer_fix") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone()) {
                    return Some(expr);
                }
            }
        }
    }
    None
}

// Helper function to extract #[name("...")]
fn extract_name_attribute(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident("name") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(lit) = syn::parse2::<Lit>(meta_list.tokens.clone()) {
                    if let Lit::Str(lit_str) = lit {
                        return Some(lit_str.value());
                    }
                }
            }
        }
    }
    None
}

// Helper function to extract #[var(...)]
fn extract_var_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("var") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone()) {
                    return Some(expr);
                }
            }
        }
    }
    None
}

// Helper function to extract #[range(...)]
fn extract_range_attribute(attrs: &[Attribute]) -> Option<syn::Expr> {
    for attr in attrs {
        if attr.path().is_ident("range") {
            if let Meta::List(meta_list) = &attr.meta {
                if let Ok(expr) = syn::parse2::<Expr>(meta_list.tokens.clone()) {
                    return Some(expr);
                }
            }
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
        dsl_name,
        fields,
        var_type,
        fix,
    }
}

fn generate_eval_var_impl(
    enum_name: &syn::Ident,
    variants: &[VariantInfo],
    env_type: Option<&syn::Type>,
) -> proc_macro2::TokenStream {
    // Generate field_schema implementation
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

    // Generate vars implementation - now generic!
    let vars_generation = generate_vars_impl(enum_name, variants);

    // Generate fix implementation
    let fix_arms = variants.iter().map(|info| {
        let variant_name = &info.variant_name;

        // Generate pattern matching for fields
        let (pattern, checks_and_output) = generate_fix_pattern_and_checks_and_output(info);

        quote! {
            #enum_name::#variant_name #pattern => {
                #checks_and_output
            }
        }
    });

    // Collect all unique object types (same logic as in generate_try_from_impl)
    let mut object_types = std::collections::HashSet::new();
    for variant in variants {
        for field in &variant.fields {
            if let Type::Path(type_path) = &field.ty {
                let segment = type_path.path.segments.last().unwrap();
                let type_name = segment.ident.to_string();
                if type_name != "i32" && type_name != "bool" {
                    object_types.insert(field.ty.clone());
                }
            }
        }
    }

    // Generate where clause for all object types
    let where_clauses = object_types.iter().map(|ty| {
        quote! {
            #ty: TryFrom<__T, Error = ::collo_ml::traits::TypeConversionError>
        }
    });

    // Generate impl based on whether env_type is specified
    if let Some(env_ty) = env_type {
        // Env-specific implementation with concrete env type
        let where_clauses_vec: Vec<_> = where_clauses.collect();
        let where_text = if where_clauses_vec.is_empty() {
            quote! {
                where
                    __T: ::collo_ml::EvalObject<Env = #env_ty>
            }
        } else {
            quote! {
                where
                    __T: ::collo_ml::EvalObject<Env = #env_ty>,
                    #(#where_clauses_vec),*
            }
        };

        quote! {
            impl<__T> ::collo_ml::EvalVar<__T> for #enum_name
                #where_text
            {
                fn field_schema() -> ::std::collections::HashMap<String, Vec<::collo_ml::traits::FieldType>> {
                    let mut schema = ::std::collections::HashMap::new();
                    #(#field_schema_entries)*
                    schema
                }

                fn vars(
                    env: &#env_ty
                ) -> ::std::result::Result<
                    ::std::collections::BTreeMap<Self, ::collomatique_ilp::Variable>,
                    ::std::any::TypeId
                > {
                    #vars_generation
                }

                fn fix(&self, env: &#env_ty) -> Option<f64> {
                    match self {
                        #(#fix_arms,)*
                    }
                }
            }
        }
    } else {
        // Generic implementation
        let where_text = if where_clauses.len() == 0 {
            quote! {}
        } else {
            quote! {
                where
                    #(#where_clauses),*
            }
        };

        quote! {
            impl<__T: ::collo_ml::EvalObject> ::collo_ml::EvalVar<__T> for #enum_name
                #where_text
            {
                fn field_schema() -> ::std::collections::HashMap<String, Vec<::collo_ml::traits::FieldType>> {
                    let mut schema = ::std::collections::HashMap::new();
                    #(#field_schema_entries)*
                    schema
                }

                fn vars(
                    env: &__T::Env
                ) -> ::std::result::Result<
                    ::std::collections::BTreeMap<Self, ::collomatique_ilp::Variable>,
                    ::std::any::TypeId
                > {
                    #vars_generation
                }

                fn fix(&self, env: &__T::Env) -> Option<f64> {
                    match self {
                        #(#fix_arms,)*
                    }
                }
            }
        }
    }
}

fn generate_field_type_expr(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => quote! { ::collo_ml::traits::FieldType::Int },
                "bool" => quote! { ::collo_ml::traits::FieldType::Bool },
                "Vec" => panic!("List are not supported as variable parameters: {:?}", ty),
                _ => {
                    // It's an object type - use TypeId
                    quote! { ::collo_ml::traits::FieldType::Object(::std::any::TypeId::of::<#ty>()) }
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
        let mut vars = ::std::collections::BTreeMap::new();
        #(#variant_iterations)*
        Ok(vars)
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
            if new_var.fix(env).is_some() {
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
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => {
                    if let Some(range_expr) = range {
                        quote! {
                            for #var_name in #range_expr
                        }
                    } else {
                        panic!("i32 fields must have a #[range(...)] attribute");
                    }
                }
                "bool" => {
                    if range.is_some() {
                        panic!("#[range(...)] attribute is not supported for bool type");
                    }
                    quote! {
                        for #var_name in [false, true]
                    }
                }
                _ => {
                    if range.is_some() {
                        panic!("#[range(...)] attribute is not supported for object types");
                    }
                    // It's an object type
                    // Get the type name from TypeId, then get all objects of that type
                    quote! {
                        for #var_name in {
                            let type_id = ::std::any::TypeId::of::<#ty>();
                            let type_name = __T::type_id_to_name(type_id.clone())
                                .map_err(|_| type_id)?;
                            __T::objects_with_typ(env, &type_name)
                                .into_iter()
                                .map(|obj| <#ty>::try_from(obj).expect("Consistent TryFrom implementation with type_id_to_name"))
                        }
                    }
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

                // Generate range check for i32 fields
                if let Type::Path(type_path) = &field.ty {
                    let segment = type_path.path.segments.last().unwrap();
                    let type_name = segment.ident.to_string();

                    if type_name == "i32" {
                        if let Some(range_expr) = &field.range {
                            checks.push(quote! {
                                if !(#range_expr).contains(#var_name) {
                                    return Some(#fix_with);
                                }
                            });
                        }
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

    // Collect all unique object types used across all variants
    let mut object_types = std::collections::HashSet::new();
    for variant in variants {
        for field in &variant.fields {
            if let Type::Path(type_path) = &field.ty {
                let segment = type_path.path.segments.last().unwrap();
                let type_name = segment.ident.to_string();
                if type_name != "i32" && type_name != "bool" {
                    object_types.insert(field.ty.clone());
                }
            }
        }
    }

    // Generate where clause for all object types
    let where_clauses = object_types.iter().map(|ty| {
        quote! {
            #ty: TryFrom<__T>
        }
    });

    let where_text = if where_clauses.len() == 0 {
        quote! {}
    } else {
        quote! {
            where
                #(#where_clauses),*
        }
    };

    quote! {
        impl<__T: ::collo_ml::EvalObject> TryFrom<&::collo_ml::eval::ExternVar<__T>> for #enum_name
            #where_text
        {
            type Error = ::collo_ml::traits::VarConversionError;

            fn try_from(value: &::collo_ml::eval::ExternVar<__T>) -> Result<Self, Self::Error> {
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
    match ty {
        Type::Path(type_path) => {
            let segment = type_path.path.segments.last().unwrap();
            let type_name = segment.ident.to_string();

            match type_name.as_str() {
                "i32" => {
                    quote! {
                        let #param_name = match &value.params[#idx] {
                            ::collo_ml::ExprValue::Int(i) => *i,
                            _ => {
                                return Err(::collo_ml::traits::VarConversionError::WrongParameterType {
                                    name: #dsl_name.into(),
                                    param: #idx,
                                    expected: ::collo_ml::traits::FieldType::Int,
                                })
                            }
                        };
                    }
                }
                "bool" => {
                    quote! {
                        let #param_name = match &value.params[#idx] {
                            ::collo_ml::ExprValue::Bool(b) => *b,
                            _ => {
                                return Err(::collo_ml::traits::VarConversionError::WrongParameterType {
                                    name: #dsl_name.into(),
                                    param: #idx,
                                    expected: ::collo_ml::traits::FieldType::Bool,
                                })
                            }
                        };
                    }
                }
                _ => {
                    // It's an object type - use the where clause constraint
                    quote! {
                        let #param_name = match &value.params[#idx] {
                            ::collo_ml::ExprValue::Object(obj) => {
                                <#ty>::try_from(obj.clone())
                                    .map_err(|_| ::collo_ml::traits::VarConversionError::WrongParameterType {
                                        name: #dsl_name.into(),
                                        param: #idx,
                                        expected: ::collo_ml::traits::FieldType::Object(::std::any::TypeId::of::<#ty>()),
                                    })?
                            }
                            _ => {
                                return Err(::collo_ml::traits::VarConversionError::WrongParameterType {
                                    name: #dsl_name.into(),
                                    param: #idx,
                                    expected: ::collo_ml::traits::FieldType::Object(::std::any::TypeId::of::<#ty>()),
                                })
                            }
                        };
                    }
                }
            }
        }
        _ => panic!("Unsupported parameter type"),
    }
}
