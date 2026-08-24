use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Field, Fields, Meta, parse_macro_input, parse_quote};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    let fields = match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    name,
                    "References can only be derived for structs with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                name,
                "References can only be derived for structs with named fields",
            ));
        }
    };

    let mut fk_fields = Vec::new();
    for field in fields {
        if has_fk_attribute(field) {
            validate_fk_attribute(field)?;
            fk_fields.push(field);
        }
    }
    if fk_fields.is_empty() {
        return Err(syn::Error::new_spanned(
            name,
            "References requires at least one #[fk] field",
        ));
    }

    let idents: Vec<_> = fk_fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();
    let types: Vec<_> = fk_fields.iter().map(|field| &field.ty).collect();

    // The impl is generic over the union type K, on top of the struct's own
    // generics, with one `References<K>` bound per #[fk] field
    let mut generics = input.generics.clone();
    generics.params.insert(0, parse_quote!(K));
    {
        let where_clause = generics.make_where_clause();
        for ty in &types {
            where_clause
                .predicates
                .push(parse_quote!(#ty: ::collomatique_state::refs::References<K>));
        }
    }
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, ty_generics, _) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::collomatique_state::refs::References<K> for #name #ty_generics #where_clause {
            fn for_each_ref(&self, f: &mut dyn ::core::ops::FnMut(K)) {
                #(
                    <#types as ::collomatique_state::refs::References<K>>::for_each_ref(&self.#idents, f);
                )*
            }
        }
    })
}

fn has_fk_attribute(field: &Field) -> bool {
    field.attrs.iter().any(|attr| attr.path().is_ident("fk"))
}

fn validate_fk_attribute(field: &Field) -> syn::Result<()> {
    for attr in &field.attrs {
        if !attr.path().is_ident("fk") {
            continue;
        }
        match &attr.meta {
            Meta::Path(_) => {}
            Meta::List(_) => {
                // `name = ident` renames the joined field; it belongs to the
                // Join derive but must parse here too since #[fk] is shared
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        meta.value()?.parse::<syn::Ident>()?;
                        Ok(())
                    } else {
                        Err(meta.error("expected #[fk] or #[fk(name = identifier)]"))
                    }
                })?;
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    attr,
                    "expected #[fk] or #[fk(name = identifier)]",
                ));
            }
        }
    }
    Ok(())
}
