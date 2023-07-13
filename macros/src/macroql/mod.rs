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
    let name = ident_to_case(&name, Case::Snake);
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
        #visi async fn #name(client: reqwest::Client, vars: #name::Vars) -> anyhow::Result<#name::Sels> {
            #[derive(serde::Serialize)]
            struct Request {
                query: &'static str,
                variables: #name::Vars,
            }
            #[derive(serde::Deserialize)]
            struct Success {
                data: #name::Sels,
            }
            #[derive(serde::Deserialize)]
            struct Failure {
                errors: Vec<Error>,
            }
            #[derive(serde::Deserialize)]
            struct Error {
                message: String,
            }
            let res = client.post("https://page.kakao.com/graphql").json(&Request { query: #query_str, variables: vars }).send().await?.json::<serde_json::Value>().await?;
            if res.get("errors").is_some() {
                Err(anyhow::anyhow!("{}", serde_json::from_value::<Failure>(res)?.errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join(", ")))
            } else {
                Ok(serde_json::from_value::<Success>(res)?.data)
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
