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
                    let name = match e.name.to_string().as_str() {
                        "typ_" => Ident::new("type", e.name.span()),
                        _ => e.name.clone(),
                    };
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
            let old_name = if e.name.to_string() == "typ_" {
                Ident::new("type", e.name.span())
            } else {
                e.name.clone()
            };
            let new_name = if old_name.to_string() == "type" {
                Ident::new("typ_", old_name.span())
            } else {
                ident_to_case(&old_name, Case::Snake)
            };
            let typ_name = if e.typ_.name.clone().map(|e| e.to_string())
                == Some("QueryFromPage".to_string())
            {
                quote!(String)
            } else {
                let typ_name = e.typ_.fmt_rs(&old_name, &self.name);
                quote!(#typ_name)
            };
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
