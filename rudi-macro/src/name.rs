use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Expr, ExprCall, ExprLit, Lit, LitStr,
};

pub(crate) enum Name {
    LitStr(LitStr),
    ExprCall(ExprCall),
}

impl TryFrom<(Expr, &'static str)> for Name {
    type Error = syn::Error;

    fn try_from((expr, message): (Expr, &'static str)) -> Result<Self, Self::Error> {
        match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(lit_str),
                ..
            }) => Ok(Name::LitStr(lit_str)),
            Expr::Call(expr_call) => Ok(Name::ExprCall(expr_call)),
            _ => Err(syn::Error::new(expr.span(), message)),
        }
    }
}

impl Parse for Name {
    fn parse(input: ParseStream) -> syn::Result<Name> {
        Name::try_from((
            input.parse::<Expr>()?,
            "expected a literal string or an expression call",
        ))
    }
}

impl ToTokens for Name {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Name::LitStr(lit_str) => lit_str.to_tokens(tokens),
            Name::ExprCall(expr_call) => expr_call.to_tokens(tokens),
        }
    }
}
