use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, ExprLit, ExprPath, Lit, LitBool, Meta, MetaNameValue, Token,
};

use crate::utils;

pub(crate) struct StructOrFunctionAttribute {
    name: Option<(Span, Expr)>,
    eager_create: Option<(Span, bool)>,
    binds: Option<(Span, Vec<ExprPath>)>,
    pub(crate) async_constructor: Option<(Span, bool)>,
    auto_register: Option<(Span, bool)>,
}

impl Parse for StructOrFunctionAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<(Span, Expr)> = None;
        let mut eager_create: Option<(Span, bool)> = None;
        let mut binds: Option<(Span, Vec<ExprPath>)> = None;
        let mut async_constructor: Option<(Span, bool)> = None;
        let mut auto_register: Option<(Span, bool)> = None;

        let attr = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;

        for meta in attr {
            let meta_path = meta.path();
            let meta_path_span = meta_path.span();

            macro_rules! check_duplicate {
                ($attribute:tt) => {
                    if $attribute.is_some() {
                        return Err(syn::Error::new(
                            meta_path_span,
                            concat!(
                                "the `",
                                stringify!($attribute),
                                "` attribute can only be set once"
                            ),
                        ));
                    }
                };
            }

            macro_rules! boolean_attr {
                ($attribute:tt) => {
                    if meta_path.is_ident(stringify!($attribute)) {
                        check_duplicate!($attribute);

                        $attribute = match meta {
                            Meta::Path(_) => Some((meta_path_span, true)),
                            Meta::NameValue(MetaNameValue { value, .. }) => match value {
                                Expr::Lit(ExprLit {
                                    lit: Lit::Bool(LitBool { value, .. }),
                                    ..
                                }) => Some((meta_path_span, value)),
                                _ => {
                                    return Err(syn::Error::new(
                                        value.span(),
                                        concat!(
                                            "the value of `",
                                            stringify!($attribute),
                                            "` must be a boolean literal"
                                        ),
                                    ))
                                }
                            },
                            Meta::List(list) => {
                                return Err(syn::Error::new(
                                    list.delimiter.span().open(),
                                    concat!(
                                        "unexpected token in the `",
                                        stringify!($attribute),
                                        "` attribute"
                                    ),
                                ))
                            }
                        };

                        continue;
                    }
                };
            }

            if meta_path.is_ident("name") {
                check_duplicate!(name);

                let MetaNameValue { value, .. } = utils::require_name_value(meta)?;

                name = Some((meta_path_span, value));
                continue;
            }

            boolean_attr!(eager_create);
            boolean_attr!(async_constructor);
            boolean_attr!(auto_register);

            if meta_path.is_ident("binds") {
                check_duplicate!(binds);

                let MetaNameValue { value, .. } = utils::require_name_value(meta)?;

                let array = if let Expr::Array(array) = value {
                    array
                } else {
                    return Err(syn::Error::new(
                        value.span(),
                        "the value of `binds` must be an array",
                    ));
                };

                let mut paths = vec![];

                for expr in array.elems {
                    if let Expr::Path(path) = expr {
                        paths.push(path);
                    } else {
                        return Err(syn::Error::new(
                            expr.span(),
                            "the element in `binds` must be an expression path",
                        ));
                    }
                }

                binds = Some((meta_path_span, paths));
                continue;
            }

            return Err(syn::Error::new(
                meta_path_span,
                 "the attribute must be one of: `name`, `eager_create`, `binds`, `async_constructor`, `auto_register`",
            ));
        }

        Ok(StructOrFunctionAttribute {
            name,
            eager_create,
            binds,
            async_constructor,
            auto_register,
        })
    }
}

impl StructOrFunctionAttribute {
    pub(crate) fn simplify(&self) -> SimpleStructOrFunctionAttribute {
        let StructOrFunctionAttribute {
            name,
            eager_create,
            binds,
            async_constructor,
            auto_register,
        } = self;

        SimpleStructOrFunctionAttribute {
            name: name
                .as_ref()
                .map(|(_, name)| {
                    quote! {
                        #name
                    }
                })
                .unwrap_or(quote! {
                    ""
                }),
            eager_create: eager_create
                .map(|(_, eager_create)| eager_create)
                .unwrap_or(false),
            binds: binds
                .as_ref()
                .map(|(_, binds)| {
                    quote! {
                        #(
                            .bind(#binds)
                        )*
                    }
                })
                .unwrap_or(quote! {}),
            async_constructor: async_constructor
                .map(|(_, async_constructor)| async_constructor)
                .unwrap_or(false),
            auto_register: auto_register
                .map(|(_, auto_register)| auto_register)
                .unwrap_or(true),
        }
    }
}

pub(crate) struct SimpleStructOrFunctionAttribute {
    pub(crate) name: TokenStream,
    pub(crate) eager_create: bool,
    pub(crate) binds: TokenStream,
    pub(crate) async_constructor: bool,
    pub(crate) auto_register: bool,
}
