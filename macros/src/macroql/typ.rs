use convert_case::{Case, Casing};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    Ident, Result, Token,
};

use crate::macroql::ident_to_case;

const ROOT_VARIABLE_NO_TYPE_ERR: &str = "root variables must be given a type name";

#[derive(Default)]
pub struct Type {
    pub name: Option<Ident>,
    pub list: bool,
    pub null: bool,
}

impl Parse for Type {
    fn parse(input: ParseStream) -> Result<Self> {
        let _ = input.parse::<Token!(:)>();
        if let Ok(brackets) = syn::__private::parse_brackets(input) {
            Ok(Self {
                name: brackets.content.parse()?,
                list: true,
                null: input.parse::<Token!(?)>().is_ok(),
            })
        } else {
            Ok(Self {
                name: input.parse()?,
                list: false,
                null: input.parse::<Token!(?)>().is_ok(),
            })
        }
    }
}

impl Type {
    pub fn fmt_gq(&self) -> String {
        let mut result = self
            .name
            .as_ref()
            .expect(ROOT_VARIABLE_NO_TYPE_ERR)
            .to_string()
            .to_case(Case::Pascal);
        if self.list {
            result = format!("[{result}]")
        }
        if !self.null {
            result = format!("{result}!")
        }
        result
    }

    pub fn fmt_rs(&self, current_name: &Ident, parent_name: &Ident) -> impl ToTokens {
        let name = if let Some(name) = &self.name {
            match name.to_string().as_str() {
                "Int" => Ident::new("i32", name.span()),
                "Long" => Ident::new("i64", name.span()),
                "Float" => Ident::new("f64", name.span()),
                "Boolean" => Ident::new("bool", name.span()),
                other => Ident::new(other, name.span()),
            }
        } else {
            ident_to_case(current_name, Case::Pascal)
        };
        let mut result = quote!(#name);
        if self.is_object() {
            let parent_name = ident_to_case(parent_name, Case::Snake);
            result = quote!(#parent_name::#result)
        }
        if self.list {
            result = quote!(Vec<#result>)
        }
        if self.null {
            result = quote!(Option<#result>)
        }
        result
    }

    pub fn is_object(&self) -> bool {
        if let Some(name) = &self.name {
            match name.to_string().as_str() {
                "Int" | "Long" | "Float" | "Boolean" | "String" => false,
                _ => true,
            }
        } else {
            true
        }
    }
}
