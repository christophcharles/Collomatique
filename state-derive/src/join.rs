use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Field, Fields, Ident, Meta, Type, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;
    let vis = &input.vis;

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "Join cannot be derived for generic types",
        ));
    }

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "Join can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "Join can only be derived for structs with named fields",
            ));
        }
    };

    let args = extract_join_attribute(input)?;
    let error_ty = &args.error;
    let output_name = args
        .output
        .clone()
        .unwrap_or_else(|| format_ident!("Joined{}", name));

    let mut joined_fields = Vec::new();
    let mut field_inits = Vec::new();
    let mut fk_types = Vec::new();
    for field in fields {
        let field_vis = &field.vis;
        let ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;
        match fk_attribute(field)? {
            Some(rename) => {
                let joined_ident = rename.as_ref().unwrap_or(ident);
                joined_fields.push(quote! {
                    #field_vis #joined_ident:
                        <#ty as ::collomatique_state::join::Joinable>::Output<'a>
                });
                field_inits.push(quote! {
                    #joined_ident: <#ty as ::collomatique_state::join::Join<Ctx>>::join(
                        &self.#ident,
                        ctx,
                    )
                    .map_err(
                        <#error_ty as ::core::convert::From<
                            <#ty as ::collomatique_state::join::Joinable>::Error,
                        >>::from,
                    )?
                });
                fk_types.push(ty);
            }
            None => {
                joined_fields.push(quote! { #field_vis #ident: &'a #ty });
                field_inits.push(quote! { #ident: &self.#ident });
            }
        }
    }
    if fk_types.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "Join requires at least one #[fk] field",
        ));
    }

    let doc = format!(
        "Joined view of [`{}`], borrowing every referenced entity from the join context",
        name
    );

    Ok(quote! {
        #[doc = #doc]
        #[derive(::core::fmt::Debug, ::core::clone::Clone)]
        #vis struct #output_name<'a> {
            #(#joined_fields,)*
        }

        impl ::collomatique_state::join::Joinable for #name {
            type Output<'a>
                = #output_name<'a>
            where
                Self: 'a;
            type Error = #error_ty;
        }

        impl<Ctx> ::collomatique_state::join::Join<Ctx> for #name
        where
            #(
                #fk_types: ::collomatique_state::join::Join<Ctx>,
                #error_ty: ::core::convert::From<
                    <#fk_types as ::collomatique_state::join::Joinable>::Error,
                >,
            )*
        {
            fn join<'a>(
                &'a self,
                ctx: &'a Ctx,
            ) -> ::core::result::Result<#output_name<'a>, #error_ty> {
                ::core::result::Result::Ok(#output_name {
                    #(#field_inits,)*
                })
            }
        }
    })
}

struct JoinArgs {
    error: Type,
    output: Option<Ident>,
}

fn extract_join_attribute(input: &DeriveInput) -> syn::Result<JoinArgs> {
    let mut error = None;
    let mut output = None;
    for attr in &input.attrs {
        if !attr.path().is_ident("join") {
            continue;
        }
        let Meta::List(_) = &attr.meta else {
            return Err(syn::Error::new_spanned(
                attr,
                "expected #[join(error = Type)] or #[join(error = Type, output = Name)]",
            ));
        };
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("error") {
                if error.is_some() {
                    return Err(meta.error("duplicate `error` argument"));
                }
                error = Some(meta.value()?.parse::<Type>()?);
                Ok(())
            } else if meta.path.is_ident("output") {
                if output.is_some() {
                    return Err(meta.error("duplicate `output` argument"));
                }
                output = Some(meta.value()?.parse::<Ident>()?);
                Ok(())
            } else {
                Err(meta.error(
                    "expected #[join(error = Type)] or #[join(error = Type, output = Name)]",
                ))
            }
        })?;
    }
    let Some(error) = error else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "Join requires a #[join(error = Type)] attribute",
        ));
    };
    Ok(JoinArgs { error, output })
}

/// Distinguishes non-`#[fk]` fields (`None`), plain `#[fk]` fields
/// (`Some(None)`) and renamed `#[fk(name = ident)]` fields (`Some(Some(ident))`)
fn fk_attribute(field: &Field) -> syn::Result<Option<Option<Ident>>> {
    let mut found = None;
    for attr in &field.attrs {
        if !attr.path().is_ident("fk") {
            continue;
        }
        match &attr.meta {
            Meta::Path(_) => {
                found = Some(None);
            }
            Meta::List(_) => {
                let mut rename = None;
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        rename = Some(meta.value()?.parse::<Ident>()?);
                        Ok(())
                    } else {
                        Err(meta.error("expected #[fk] or #[fk(name = identifier)]"))
                    }
                })?;
                found = Some(rename);
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected #[fk] or #[fk(name = identifier)]",
                ));
            }
        }
    }
    Ok(found)
}
