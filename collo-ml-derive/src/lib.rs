use proc_macro::TokenStream;

mod view_object;

#[proc_macro_derive(ViewObject, attributes(eval_object, hidden, pretty))]
pub fn derive_view_object(input: TokenStream) -> TokenStream {
    view_object::derive(input)
}
