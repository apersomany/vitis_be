use convert_case::Case;
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result, Token,
};

use crate::macroql::ident_to_case;

use super::{parse_curly, parse_round, typ::Type};

const INLINE_NO_TYPE_ERR: &str = "inline fragments must be given a type name";
const INLINE_NO_NEST_ERR: &str = "inline fragment nesting is not supported";

pub enum Sel {
    Normal {
        name: Ident,
        args: Vec<Ident>,
        typ_: Type,
        flds: Vec<Self>,
    },
    Inline {
        typ_: Type,
        flds: Vec<Self>,
    },
}

impl Parse for Sel {
    fn parse(input: ParseStream) -> Result<Self> {
        if input.parse::<Token!(...)>().is_ok() {
            Ok(Self::Inline {
                typ_: input.parse()?,
                flds: parse_curly(input)?,
            })
        } else {
            Ok(Self::Normal {
                name: input.parse()?,
                args: parse_round(input)?,
                typ_: input.parse()?,
                flds: parse_curly(input)?,
            })
        }
    }
}

impl Sel {
    pub fn fmt_gq(&self) -> String {
        match self {
            Self::Normal {
                name, args, flds, ..
            } => {
                let args = if args.len() > 0 {
                    let args = args
                        .iter()
                        .map(|e| format!("{e}: ${e}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({args})")
                } else {
                    format!("")
                };
                if flds.len() > 0 {
                    let typn = if flds.iter().any(|e| match e {
                        Self::Inline { .. } => true,
                        Self::Normal { .. } => false,
                    }) {
                        format!(", __typename")
                    } else {
                        format!("")
                    };
                    let flds = flds
                        .iter()
                        .map(|e| e.fmt_gq())
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("{name}{args} {{ {flds}{typn} }}")
                } else {
                    format!("{name}{args}")
                }
            }
            Self::Inline { flds, typ_ } => {
                let typ_ = typ_.name.as_ref().expect(INLINE_NO_TYPE_ERR);
                let flds = flds
                    .iter()
                    .map(|e| e.fmt_gq())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("... on {typ_} {{ {flds} }}",)
            }
        }
    }

    pub fn fmt_rs(&self) -> impl ToTokens {
        match self {
            Self::Normal {
                name, typ_, flds, ..
            } => {
                let mut normal = Vec::new();
                let mut inline = Vec::new();
                for e in flds {
                    match e {
                        Self::Normal {
                            name: old_name,
                            typ_,
                            ..
                        } => {
                            normal.push((old_name, typ_));
                        }
                        Self::Inline { typ_, flds } => {
                            inline.push((typ_, flds));
                        }
                    }
                }
                let typ_name = typ_.name.as_ref().unwrap_or(name);
                let typ_name = ident_to_case(typ_name, Case::Pascal);
                let typ_defs = if inline.len() > 0 {
                    let variants = inline.into_iter().map(|(typ_, flds)| {
                        let var_name = typ_.name.as_ref().expect(INLINE_NO_TYPE_ERR);
                        let var_name = ident_to_case(var_name, Case::Pascal);
                        let fields = flds
                            .iter()
                            .map(|e| match e {
                                Self::Normal {
                                    name: old_name,
                                    typ_,
                                    ..
                                } => (old_name, typ_),
                                Self::Inline { .. } => unimplemented!("{INLINE_NO_NEST_ERR}"),
                            })
                            .chain(normal.clone().into_iter())
                            .map(|(old_name, typ_)| {
                                let new_name = ident_to_case(old_name, Case::Snake);
                                let typ_name = typ_.fmt_rs(old_name, name);
                                let old_name = old_name.to_string();
                                quote!(#[serde(rename = #old_name)] #new_name: #typ_name)
                            });
                        quote! {
                            #var_name {
                                #(#fields,)*
                            }
                        }
                    });
                    quote! {
                        #[derive(Debug, serde::Deserialize)]
                        #[serde(tag = "__typename")]
                        pub enum #typ_name {
                            #(#variants,)*
                            #[serde(other)]
                            Unknown
                        }
                    }
                } else {
                    let fields = normal.into_iter().map(|(old_name, typ_)| {
                        let new_name = ident_to_case(old_name, Case::Snake);
                        let typ_name = typ_.fmt_rs(old_name, name);
                        let old_name = old_name.to_string();
                        quote!(#[serde(rename = #old_name)] pub #new_name: #typ_name)
                    });
                    quote! {
                        #[derive(Debug, serde::Deserialize)]
                        pub struct #typ_name {
                            #(#fields,)*
                        }
                    }
                };
                let mod_name = ident_to_case(&typ_name, Case::Snake);
                let mod_defs = flds.iter().filter(|e| e.is_object()).map(|e| e.fmt_rs());
                let mod_defs = quote! {
                    pub mod #mod_name {
                        #(#mod_defs)*
                    }
                };
                quote! {
                    #typ_defs
                    #mod_defs
                }
            }
            Self::Inline { flds, .. } => {
                let mod_defs = flds.iter().filter(|e| e.is_object()).map(|e| e.fmt_rs());
                quote!(#(#mod_defs)*)
            }
        }
    }

    fn is_object(&self) -> bool {
        match self {
            Self::Normal { typ_, .. } => typ_,
            Self::Inline { typ_, .. } => typ_,
        }
        .is_object()
    }
}
