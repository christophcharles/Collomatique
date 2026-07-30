//! The `ContentOrd` and `ContentIdentity` derives
//!
//! `#[derive(ContentOrd)]` implements the document order as the **product of
//! all fields** (design doc §8, step 6.5): the macro walks every field by
//! construction, so a forgotten field is impossible and a new field whose
//! type has no `ContentOrd` impl is a compile error that forces a decision.
//! Four field attributes override the default per-type dispatch — see
//! [FieldRule].
//!
//! `#[derive(ContentIdentity)]` emits the marker asserting that `==`
//! coincides with content equivalence, after checking everything a macro can
//! check (see [derive_identity]).

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Attribute, Data, DeriveInput, Expr, Field, Fields, FieldsNamed, Ident, Token};

/// The comparison rule of one field, from its optional `#[ord(...)]`
/// attribute.
enum FieldRule {
    /// No attribute: dispatch through `ContentOrd` on the field's type.
    Default,
    /// `#[ord(atom)]`: inline discrete comparison (`discrete`, whose `Eq`
    /// bound enforces the reflexivity obligation on the field's type).
    Atom,
    /// `#[ord(ignore)]`: the order does not see this field — the
    /// structural source of equivalence classes.
    Ignore,
    /// `#[ord(total)]`: the field's native total order is its content
    /// order (`Ord::cmp`, self-enforcing — see [cmp_expr]). The field's
    /// `Ord` must itself be well-founded (no infinite strictly-descending
    /// chain): integers are, `String` is **not** (`"b" > "ab" > "aab" >
    /// …`) — a non-well-founded `total` field silently voids the
    /// termination proof. This cannot be checked mechanically; it is part
    /// of the field's design decision.
    Total,
    /// `#[ord(with = <expr>)]`: call the expression (path or closure).
    With(Expr),
}

impl Parse for FieldRule {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "atom" => Ok(FieldRule::Atom),
            "ignore" => Ok(FieldRule::Ignore),
            "total" => Ok(FieldRule::Total),
            // A plain expression, not a string: `with = option_lift_discrete`
            // and `with = |a, b| …` both parse (house style, like
            // `#[join(error = NewId)]`).
            "with" => {
                input.parse::<Token![=]>()?;
                Ok(FieldRule::With(input.parse()?))
            }
            _ => Err(syn::Error::new(
                ident.span(),
                "expected `atom`, `ignore`, `total` or `with = <expression>`",
            )),
        }
    }
}

/// Extracts the rule of one field; at most one `#[ord(...)]` per field.
fn field_rule(attrs: &[Attribute]) -> syn::Result<FieldRule> {
    let mut found: Option<FieldRule> = None;
    for attr in attrs {
        if !attr.path().is_ident("ord") {
            continue;
        }
        if found.is_some() {
            return Err(syn::Error::new(
                attr.span(),
                "at most one `#[ord(...)]` attribute per field",
            ));
        }
        found = Some(attr.parse_args()?);
    }
    Ok(found.unwrap_or(FieldRule::Default))
}

/// Both derives accept only non-generic types: a generic impl would need
/// bounds the macro cannot infer, and the one generic case in the codebase
/// (`NonEmptyRangeInclusive`) is hand-written on purpose.
fn reject_generics(input: &DeriveInput) -> syn::Result<()> {
    if input.generics.params.is_empty() {
        return Ok(());
    }
    Err(syn::Error::new_spanned(
        &input.generics,
        "ContentOrd and ContentIdentity cannot be derived for generic types; \
         write the impl by hand",
    ))
}

/// Every named field of the type: a struct's own, or the union of all
/// variants' (unit variants contribute none). Tuple shapes and unions are
/// rejected here, once, for both derives.
fn all_named_fields(input: &DeriveInput) -> syn::Result<Vec<&Field>> {
    match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => Ok(fields.named.iter().collect()),
            Fields::Unit => Ok(Vec::new()),
            Fields::Unnamed(fields) => Err(syn::Error::new(
                fields.span(),
                "ContentOrd and ContentIdentity do not support tuple structs",
            )),
        },
        Data::Enum(data) => {
            let mut all = Vec::new();
            for variant in &data.variants {
                match &variant.fields {
                    Fields::Named(fields) => all.extend(fields.named.iter()),
                    Fields::Unit => {}
                    Fields::Unnamed(fields) => {
                        return Err(syn::Error::new(
                            fields.span(),
                            "ContentOrd and ContentIdentity do not support tuple variants",
                        ));
                    }
                }
            }
            Ok(all)
        }
        Data::Union(data) => Err(syn::Error::new(
            data.union_token.span(),
            "ContentOrd and ContentIdentity do not support unions",
        )),
    }
}

/// The generated comparison of one field. `lhs`/`rhs` are the borrowed
/// access expressions — `&self.x` / `&other.x` for structs, the match
/// bindings for enums (which are already references, by match ergonomics).
fn cmp_expr(rule: &FieldRule, lhs: TokenStream, rhs: TokenStream) -> TokenStream {
    match rule {
        FieldRule::Default => quote! {
            ::collomatique_state::partial_order::ContentOrd::content_cmp(#lhs, #rhs)
        },
        FieldRule::Atom => quote! {
            ::collomatique_state::partial_order::discrete(#lhs, #rhs)
        },
        // Constant Equal: the order does not see the field. This is what
        // makes the containing type's content equivalence coarser than its
        // `==` — the declared quotient.
        FieldRule::Ignore => quote! {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        },
        // `Ord::cmp`, not `partial_cmp`: the call itself demands `Ord`,
        // whose contract (`Ord: Eq`, `cmp(x, x) == Equal`, `Equal` iff
        // `==`) is exactly what makes the field's native order a valid,
        // reflexive content order. A genuinely partial order goes through
        // `with` instead.
        FieldRule::Total => quote! {
            ::core::option::Option::Some(::core::cmp::Ord::cmp(#lhs, #rhs))
        },
        FieldRule::With(expr) => quote! { (#expr)(#lhs, #rhs) },
    }
}

/// The product over all fields, in declaration order. An empty struct
/// short-circuits: an empty array literal would not infer its item type.
fn struct_body(fields: &FieldsNamed) -> syn::Result<TokenStream> {
    if fields.named.is_empty() {
        return Ok(quote! {
            ::core::option::Option::Some(::core::cmp::Ordering::Equal)
        });
    }
    let cmps = fields
        .named
        .iter()
        .map(|field| {
            let rule = field_rule(&field.attrs)?;
            let name = field.ident.as_ref().expect("named field");
            Ok(cmp_expr(
                &rule,
                quote! { &self.#name },
                quote! { &other.#name },
            ))
        })
        .collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        ::collomatique_state::partial_order::combine([#(#cmps),*])
    })
}

/// One arm per *same*-variant pair, destructuring both sides with distinct
/// bindings; a unit variant is the empty product. The trailing `_ => None`
/// (different variants are incomparable) is emitted only when the enum has
/// at least two variants — on a single-variant enum it would be unreachable
/// and warn.
fn enum_body(data: &syn::DataEnum) -> syn::Result<TokenStream> {
    let mut arms = Vec::new();
    for variant in &data.variants {
        let v = &variant.ident;
        match &variant.fields {
            Fields::Unit => arms.push(quote! {
                (Self::#v, Self::#v) =>
                    ::core::option::Option::Some(::core::cmp::Ordering::Equal),
            }),
            Fields::Named(fields) => {
                let names: Vec<&Ident> = fields
                    .named
                    .iter()
                    .map(|f| f.ident.as_ref().expect("named field"))
                    .collect();
                let self_bind: Vec<Ident> =
                    names.iter().map(|n| format_ident!("self_{}", n)).collect();
                let other_bind: Vec<Ident> =
                    names.iter().map(|n| format_ident!("other_{}", n)).collect();
                let cmps = fields
                    .named
                    .iter()
                    .zip(self_bind.iter().zip(&other_bind))
                    .map(|(field, (s, o))| {
                        let rule = field_rule(&field.attrs)?;
                        Ok(cmp_expr(&rule, quote! { #s }, quote! { #o }))
                    })
                    .collect::<syn::Result<Vec<_>>>()?;
                arms.push(quote! {
                    (
                        Self::#v { #(#names: #self_bind),* },
                        Self::#v { #(#names: #other_bind),* },
                    ) => ::collomatique_state::partial_order::combine([#(#cmps),*]),
                });
            }
            Fields::Unnamed(fields) => {
                return Err(syn::Error::new(
                    fields.span(),
                    "#[derive(ContentOrd)] does not support tuple variants",
                ));
            }
        }
    }
    let fallback = (data.variants.len() > 1).then(|| quote! { _ => ::core::option::Option::None, });
    Ok(quote! {
        match (self, other) {
            #(#arms)*
            #fallback
        }
    })
}

/// `#[derive(ContentOrd)]`: the document order as the product of all fields.
pub fn derive(input: DeriveInput) -> syn::Result<TokenStream> {
    reject_generics(&input)?;
    let ident = &input.ident;
    let body = match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(fields) => struct_body(fields)?,
            Fields::Unit => quote! {
                ::core::option::Option::Some(::core::cmp::Ordering::Equal)
            },
            Fields::Unnamed(f) => {
                return Err(syn::Error::new(
                    f.span(),
                    "#[derive(ContentOrd)] does not support tuple structs",
                ));
            }
        },
        Data::Enum(e) => enum_body(e)?,
        Data::Union(u) => {
            return Err(syn::Error::new(
                u.union_token.span(),
                "#[derive(ContentOrd)] does not support unions",
            ));
        }
    };
    Ok(quote! {
        impl ::collomatique_state::partial_order::ContentOrd for #ident {
            fn content_cmp(
                &self,
                other: &Self,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                #body
            }
        }
    })
}

/// `#[derive(ContentIdentity)]`: asserts that `==` coincides with content
/// equivalence for this type, so containers may match it by `==`/`Ord`.
///
/// The macro verifies what it can see: every field's rule must preserve
/// identity — `ignore` is rejected outright (an ignored field IS a content
/// quotient), `with` is rejected as unanalyzable (hand-write the marker
/// impl if the custom rule preserves identity), default fields must
/// themselves be `ContentIdentity`, atom fields must be `Eq`, and `total`
/// fields are safe by `Ord`'s own contract (`Equal` iff `==`).
///
/// ONE premise remains that no macro can verify: this type's `==` must be
/// the structural, field-wise equality — in practice, `PartialEq` must be
/// *derived*, in the same derive list where this marker is requested. The
/// compiler strips `#[derive(...)]` lists from macro input, so the macro
/// literally cannot see whether that is the case, and must never sign a
/// claim it cannot check. That co-location is the audit trail: replacing the
/// derived `PartialEq` with a hand-written one obligates re-justifying the
/// `ContentIdentity` right next to it.
pub fn derive_identity(input: DeriveInput) -> syn::Result<TokenStream> {
    reject_generics(&input)?;
    let mut asserts = Vec::new();
    for field in all_named_fields(&input)? {
        let ty = &field.ty;
        match field_rule(&field.attrs)? {
            FieldRule::Default => asserts.push(quote! {
                assert_content_identity::<#ty>();
            }),
            FieldRule::Atom => asserts.push(quote! {
                assert_eq_impl::<#ty>();
            }),
            // Safe by Ord's contract: Ord: Eq, and cmp == Equal iff ==.
            FieldRule::Total => {}
            FieldRule::Ignore => {
                return Err(syn::Error::new(
                    field.span(),
                    "an `#[ord(ignore)]`d field is a content quotient: \
                     this type cannot be ContentIdentity",
                ));
            }
            FieldRule::With(_) => {
                return Err(syn::Error::new(
                    field.span(),
                    "`#[ord(with = ...)]` cannot be analyzed by the derive; \
                     write the ContentIdentity impl by hand if the custom \
                     rule preserves identity",
                ));
            }
        }
    }
    let ident = &input.ident;
    // The static-assert pattern, not `where`-clauses on the impl: failures
    // are deterministic compile errors with a clear span, and the
    // `ContentIdentity: Eq` supertrait already forces `#ident: Eq` through
    // the emitted impl itself.
    Ok(quote! {
        impl ::collomatique_state::partial_order::ContentIdentity for #ident {}
        const _: () = {
            #[allow(dead_code)]
            fn assert_content_identity<
                T: ::collomatique_state::partial_order::ContentIdentity,
            >() {
            }
            #[allow(dead_code)]
            fn assert_eq_impl<T: ::core::cmp::Eq>() {}
            #[allow(dead_code)]
            fn asserts() {
                #(#asserts)*
            }
        };
    })
}
