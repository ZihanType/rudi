use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, Meta, MetaList, MetaNameValue, Path, Token, Type,
};

use crate::utils::{require_list, require_name_value};

// #[di(
//     name = "..",
//     option(T),
//     default = 42,
//     vector(T),
// )]

pub(crate) struct FieldOrArgumentAttribute {
    name: Option<(Path, Expr)>,
    option: Option<(Path, Type)>,
    default: Option<(Path, Expr)>,
    vector: Option<(Path, Type)>,
}

impl Parse for FieldOrArgumentAttribute {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut name: Option<(Path, Expr)> = None;
        let mut option: Option<(Path, Type)> = None;
        let mut default: Option<(Path, Expr)> = None;
        let mut vector: Option<(Path, Type)> = None;

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

            if meta_path.is_ident("name") {
                check_duplicate!(name);

                let MetaNameValue { path, value, .. } = require_name_value(meta)?;

                name = Some((path, value));
                continue;
            }

            if meta_path.is_ident("option") {
                check_duplicate!(option);

                let MetaList { path, tokens, .. } = require_list(meta)?;

                let ty = syn::parse2::<Type>(tokens)?;
                option = Some((path, ty));
                continue;
            }

            if meta_path.is_ident("default") {
                check_duplicate!(default);

                default = match meta {
                    Meta::Path(path) => Some((
                        path,
                        syn::parse2(quote!(::core::default::Default::default()))?,
                    )),
                    Meta::NameValue(MetaNameValue { path, value, .. }) => Some((path, value)),
                    Meta::List(list) => {
                        let span = list.delimiter.span().open();
                        return Err(syn::Error::new(
                            span,
                            "unexpected token in `default` attribute",
                        ));
                    }
                };
                continue;
            }

            if meta_path.is_ident("vector") {
                check_duplicate!(vector);

                let MetaList { path, tokens, .. } = require_list(meta)?;

                let ty = syn::parse2::<Type>(tokens)?;
                vector = Some((path, ty));
                continue;
            }

            return Err(syn::Error::new(
                meta_path_span,
                "the attribute must be one of: `name`, `option`, `default`, `vector`",
            ));
        }

        if let (Some(_), Some((vector, _))) = (&name, &vector) {
            return Err(syn::Error::new(
                vector.span(),
                "the `name` and `vector` attributes cannot be used together",
            ));
        }

        match (&option, &default, &vector) {
            (Some(_), Some(_), Some((vector, _))) => {
                return Err(syn::Error::new(
                    vector.span(),
                    "the `option`, `default`, and `vector` attributes cannot be used together",
                ));
            }
            (Some(_), Some((default, _)), None) => {
                return Err(syn::Error::new(
                    default.span(),
                    "the `option` and `default` attributes cannot be used together",
                ));
            }
            (Some(_), None, Some((vector, _))) => {
                return Err(syn::Error::new(
                    vector.span(),
                    "the `option` and `vector` attributes cannot be used together",
                ));
            }
            (None, Some(_), Some((vector, _))) => {
                return Err(syn::Error::new(
                    vector.span(),
                    "the `default` and `vector` attributes cannot be used together",
                ));
            }
            _ => {}
        }

        Ok(FieldOrArgumentAttribute {
            name,
            option,
            default,
            vector,
        })
    }
}

impl FieldOrArgumentAttribute {
    pub(crate) fn simplify(self) -> SimpleFieldOrArgumentAttribute {
        let FieldOrArgumentAttribute {
            name,
            option,
            default,
            vector,
        } = self;

        SimpleFieldOrArgumentAttribute {
            name: name.map(|(_, expr)| quote!(#expr)).unwrap_or(quote!("")),
            option: option.map(|(_, ty)| ty),
            default: default.map(|(_, expr)| expr),
            vector: vector.map(|(_, ty)| ty),
        }
    }
}

pub(crate) struct SimpleFieldOrArgumentAttribute {
    pub(crate) name: TokenStream,
    pub(crate) option: Option<Type>,
    pub(crate) default: Option<Expr>,
    pub(crate) vector: Option<Type>,
}
