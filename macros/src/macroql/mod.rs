use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, Result, Token, Visibility,
};

use self::{sel::Sel, typ::Type, var::Var};

pub mod sel;
pub mod typ;
pub mod var;

pub fn macroql(input: TokenStream) -> TokenStream {
    let MQL {
        visi,
        oper,
        name,
        vars,
        sels,
    } = parse_macro_input!(input);
    let vars = Var {
        name: format_ident!("vars"),
        typ_: Type::default(),
        flds: vars,
    };
    let sels = Sel::Normal {
        name: format_ident!("sels"),
        args: Vec::new(),
        typ_: Type::default(),
        flds: sels,
    };
    let vars_fmt_gq = vars.fmt_gq();
    let vars_fmt_rs = vars.fmt_rs();
    let sels_fmt_gq = &sels.fmt_gq()[4..];
    let sels_fmt_rs = sels.fmt_rs();
    let query_str = format!("{oper} {name}{vars_fmt_gq}{sels_fmt_gq}");
    quote! {
        impl Client {
            #visi async fn #name(&self, vars: #name::Vars) -> anyhow::Result<#name::Sels> {
                self.req(#query_str, vars).await
            }
        }

        #visi mod #name {
            #vars_fmt_rs
            #sels_fmt_rs
        }
    }
    .into()
}

pub struct MQL {
    visi: Visibility,
    oper: Ident,
    name: Ident,
    vars: Vec<Var>,
    sels: Vec<Sel>,
}

impl Parse for MQL {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            visi: input.parse()?,
            oper: input.parse()?,
            name: input.parse()?,
            vars: parse_round(input)?,
            sels: parse_curly(input)?,
        })
    }
}

pub fn ident_to_case(ident: &Ident, case: Case) -> Ident {
    Ident::new(&ident.to_string().to_case(case), ident.span())
}

pub fn parse_curly<T: Parse>(input: ParseStream) -> Result<Vec<T>> {
    Ok(if let Ok(brackets) = syn::__private::parse_braces(input) {
        Punctuated::<T, Token!(,)>::parse_terminated(&brackets.content)?
            .into_iter()
            .collect()
    } else {
        Vec::new()
    })
}

pub fn parse_round<T: Parse>(input: ParseStream) -> Result<Vec<T>> {
    Ok(if let Ok(brackets) = syn::__private::parse_parens(input) {
        Punctuated::<T, Token!(,)>::parse_terminated(&brackets.content)?
            .into_iter()
            .collect()
    } else {
        Vec::new()
    })
}