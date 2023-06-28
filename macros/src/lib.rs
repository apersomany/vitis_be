use proc_macro::TokenStream;

mod macroql;

#[proc_macro]
pub fn macroql(input: TokenStream) -> TokenStream {
    macroql::macroql(input)
}
