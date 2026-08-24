use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, Meta, Type, parse_macro_input};

pub fn derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match expand(&input) {
        Ok(tokens) => TokenStream::from(tokens),
        Err(err) => TokenStream::from(err.to_compile_error()),
    }
}

fn expand(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let name = &input.ident;

    if !input.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "EntityId cannot be derived for generic types",
        ));
    }

    if !is_u64_newtype(&input.data) {
        return Err(syn::Error::new_spanned(
            name,
            "EntityId can only be derived for tuple structs with a single u64 field",
        ));
    }

    // These leaf impls must stay concrete per-ID: a blanket impl in
    // `generic/state/` would overlap with the container lifts
    let join_impls = extract_entity_attribute(&input.attrs)?.map(|entity| {
        quote! {
            impl ::collomatique_state::join::Joinable for #name {
                type Output<'a>
                    = &'a #entity
                where
                    Self: 'a;
                type Error = #name;
            }

            impl<Ctx: ::collomatique_state::join::Lookup<#name, Entity = #entity>>
                ::collomatique_state::join::Join<Ctx> for #name
            {
                fn join<'a>(&'a self, ctx: &'a Ctx) -> ::core::result::Result<&'a #entity, #name> {
                    <Ctx as ::collomatique_state::join::Lookup<#name>>::lookup(ctx, *self)
                        .ok_or(*self)
                }
            }
        }
    });

    Ok(quote! {
        impl ::collomatique_state::ids::Id for #name {
            fn inner(&self) -> u64 {
                self.0
            }

            unsafe fn new(value: u64) -> #name {
                #name(value)
            }
        }

        impl<K: ::core::convert::From<#name>> ::collomatique_state::refs::References<K> for #name {
            fn for_each_ref(&self, f: &mut dyn ::core::ops::FnMut(K)) {
                f(<K as ::core::convert::From<#name>>::from(*self));
            }
        }

        #join_impls
    })
}

fn is_u64_newtype(data: &Data) -> bool {
    let Data::Struct(data_struct) = data else {
        return false;
    };
    let Fields::Unnamed(fields) = &data_struct.fields else {
        return false;
    };
    if fields.unnamed.len() != 1 {
        return false;
    }
    matches!(
        &fields.unnamed[0].ty,
        Type::Path(type_path) if type_path.qself.is_none() && type_path.path.is_ident("u64")
    )
}

fn extract_entity_attribute(attrs: &[Attribute]) -> syn::Result<Option<Type>> {
    for attr in attrs {
        if attr.path().is_ident("entity") {
            let Meta::List(meta_list) = &attr.meta else {
                return Err(syn::Error::new_spanned(attr, "expected #[entity(Type)]"));
            };
            let ty = syn::parse2::<Type>(meta_list.tokens.clone())
                .map_err(|_| syn::Error::new_spanned(attr, "expected #[entity(Type)]"))?;
            return Ok(Some(ty));
        }
    }
    Ok(None)
}
