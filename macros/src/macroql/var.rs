use convert_case::Case;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result,
};

use crate::macroql::ident_to_case;

use super::{parse_curly, typ::Type};

pub struct Var {
    pub name: Ident,
    pub typ_: Type,
    pub flds: Vec<Self>,
}

impl Parse for Var {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            typ_: input.parse()?,
            flds: parse_curly(input)?,
        })
    }
}

impl Var {
    pub fn fmt_gq(&self) -> String {
        if self.flds.len() > 0 {
            let flds = self
                .flds
                .iter()
                .map(|e| {
                    let name = &e.name;
                    let typ_ = e.typ_.fmt_gq();
                    format!("${name}: {typ_}")
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("({flds})")
        } else {
            format!("")
        }
    }

    pub fn fmt_rs(&self) -> impl ToTokens {
        let typ_name = self.typ_.name.as_ref().unwrap_or(&self.name);
        let typ_name = ident_to_case(typ_name, Case::Pascal);
        let mod_name = ident_to_case(&typ_name, Case::Snake);
        let mod_defs = self
            .flds
            .iter()
            .filter(|e| e.typ_.is_object())
            .map(|e| e.fmt_rs());
        let fields = self.flds.iter().map(|e| {
            let old_name = &e.name;
            let new_name = ident_to_case(old_name, Case::Snake);
            let typ_name = e.typ_.fmt_rs(old_name, &self.name);
            let old_name = old_name.to_string();
            quote!(#[serde(rename = #old_name)] pub #new_name: #typ_name)
        });
        quote! {
            #[derive(Debug, serde::Serialize)]
            pub struct #typ_name {
                #(#fields,)*
            }

            pub mod #mod_name {
                #(#mod_defs)*
            }
        }
    }
}
