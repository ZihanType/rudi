use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{meta::ParseNestedMeta, parse_quote, spanned::Spanned, Attribute, Expr, Token, Type};

// #[di(
//     name = "..",
//     option = T,
//     default = 42,
//     vector = T,
// )]

#[derive(Default)]
pub(crate) struct FieldOrArgumentAttributes {
    name: Option<(Span, Expr)>,
    option: Option<(Span, Type)>,
    default: Option<(Span, Expr)>,
    vector: Option<(Span, Type)>,
}

impl FieldOrArgumentAttributes {
    fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        let meta_path = &meta.path;
        let meta_path_span = meta_path.span();

        macro_rules! check_duplicate {
            ($attribute:tt) => {
                if self.$attribute.is_some() {
                    return Err(meta.error(concat!(
                        "the `",
                        stringify!($attribute),
                        "` attribute can only be set once"
                    )));
                }
            };
        }

        if meta_path.is_ident("name") {
            check_duplicate!(name);
            self.name = Some((meta_path_span, meta.value()?.parse::<Expr>()?));
            return Ok(());
        }

        if meta_path.is_ident("option") {
            check_duplicate!(option);
            self.option = Some((meta_path_span, meta.value()?.parse::<Type>()?));
            return Ok(());
        }

        if meta_path.is_ident("default") {
            check_duplicate!(default);

            self.default = Some((
                meta_path_span,
                if meta.input.is_empty() || meta.input.peek(Token![,]) {
                    parse_quote!(::core::default::Default::default())
                } else {
                    meta.value()?.parse::<Expr>()?
                },
            ));

            return Ok(());
        }

        if meta_path.is_ident("vector") {
            check_duplicate!(vector);
            self.vector = Some((meta_path_span, meta.value()?.parse::<Type>()?));
            return Ok(());
        }

        Err(meta.error("the attribute must be one of: `name`, `option`, `default`, `vector`"))
    }
}

impl TryFrom<&Attribute> for FieldOrArgumentAttributes {
    type Error = syn::Error;

    fn try_from(attr: &Attribute) -> Result<Self, Self::Error> {
        let mut attrs = FieldOrArgumentAttributes::default();
        attr.parse_nested_meta(|meta| attrs.parse(meta))?;

        let FieldOrArgumentAttributes {
            name,
            option,
            default,
            vector,
        } = &attrs;

        if let (Some((name, _)), Some((vector, _))) = (name, vector) {
            macro_rules! err {
                ($span:expr) => {
                    syn::Error::new(
                        $span.clone(),
                        "the `name` and `vector` attributes cannot be used together",
                    )
                };
            }

            let mut e = err!(name);
            e.combine(err!(vector));

            return Err(e);
        }

        match (option, default, vector) {
            (Some((option, _)), Some((default, _)), Some((vector, _))) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option`, `default`, and `vector` attributes cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(default));
                e.combine(err!(vector));

                return Err(e);
            }
            (Some((option, _)), Some((default, _)), None) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option` and `default` attributes cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(default));

                return Err(e);
            }
            (Some((option, _)), None, Some((vector, _))) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option` and `vector` attributes cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(vector));

                return Err(e);
            }
            (None, Some((default, _)), Some((vector, _))) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `default` and `vector` attributes cannot be used together",
                        )
                    };
                }

                let mut e = err!(default);
                e.combine(err!(vector));

                return Err(e);
            }
            _ => {}
        }

        Ok(attrs)
    }
}

impl FieldOrArgumentAttributes {
    pub(crate) fn from_attrs(
        attrs: &mut Vec<Attribute>,
    ) -> syn::Result<Option<FieldOrArgumentAttributes>> {
        let mut field_or_argument_attrs = None;
        let mut errors = Vec::new();
        let mut di_already_appeared = false;

        attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            if di_already_appeared {
                let err =
                    syn::Error::new(attr.span(), "only one `#[di(...)]` attribute is allowed");
                errors.push(err);
            } else {
                match FieldOrArgumentAttributes::try_from(attr) {
                    Ok(o) => field_or_argument_attrs = Some(o),
                    Err(e) => errors.push(e),
                }
            }

            di_already_appeared = true;
            false
        });

        if let Some(e) = errors.into_iter().reduce(|mut a, b| {
            a.combine(b);
            a
        }) {
            return Err(e);
        }

        Ok(field_or_argument_attrs)
    }

    pub(crate) fn simplify(self) -> SimpleFieldOrArgumentAttributes {
        let FieldOrArgumentAttributes {
            name,
            option,
            default,
            vector,
        } = self;

        SimpleFieldOrArgumentAttributes {
            name: name
                .map(|(_, expr)| quote!(#expr))
                .unwrap_or_else(|| quote!("")),
            option: option.map(|(_, ty)| ty),
            default: default.map(|(_, expr)| expr),
            vector: vector.map(|(_, ty)| ty),
        }
    }
}

pub(crate) struct SimpleFieldOrArgumentAttributes {
    pub(crate) name: TokenStream,
    pub(crate) option: Option<Type>,
    pub(crate) default: Option<Expr>,
    pub(crate) vector: Option<Type>,
}
