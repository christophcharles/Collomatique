use proc_macro::TokenStream;

mod eval_object;
mod view_object;

#[proc_macro_derive(ViewObject, attributes(eval_object, hidden, pretty))]
pub fn derive_view_object(input: TokenStream) -> TokenStream {
    view_object::derive(input)
}

#[proc_macro_derive(EvalObject, attributes(env, cached, name))]
pub fn derive_eval_object(input: TokenStream) -> TokenStream {
    eval_object::derive(input)
}
