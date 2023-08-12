use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, spanned::Spanned, Attribute, Expr, Type};

// #[di(
//     name = "..",
//     option = T,
//     default = 42,
//     vector = T,
// )]

pub(crate) struct FieldOrArgumentAttribute {
    name: Option<(Span, Expr)>,
    option: Option<(Span, Type)>,
    default: Option<(Span, Expr)>,
    vector: Option<(Span, Type)>,
}

impl TryFrom<&Attribute> for FieldOrArgumentAttribute {
    type Error = syn::Error;

    fn try_from(attr: &Attribute) -> Result<Self, Self::Error> {
        let mut name: Option<(Span, Expr)> = None;
        let mut option: Option<(Span, Type)> = None;
        let mut default: Option<(Span, Expr)> = None;
        let mut vector: Option<(Span, Type)> = None;

        attr.parse_nested_meta(|meta| {
            macro_rules! check_duplicate {
                ($attribute:tt) => {
                    if $attribute.is_some() {
                        return Err(meta.error(concat!(
                            "the `",
                            stringify!($attribute),
                            "` attribute can only be set once"
                        )));
                    }
                };
            }

            let meta_path = &meta.path;
            let meta_path_span = meta_path.span();

            if meta_path.is_ident("name") {
                check_duplicate!(name);
                name = Some((meta_path_span, meta.value()?.parse::<Expr>()?));
                return Ok(());
            }

            if meta_path.is_ident("option") {
                check_duplicate!(option);
                option = Some((meta_path_span, meta.value()?.parse::<Type>()?));
                return Ok(());
            }

            if meta_path.is_ident("default") {
                check_duplicate!(default);

                default = Some((
                    meta_path_span,
                    if meta.input.is_empty() {
                        parse_quote!(::core::default::Default::default())
                    } else {
                        meta.value()?.parse::<Expr>()?
                    },
                ));

                return Ok(());
            }

            if meta_path.is_ident("vector") {
                check_duplicate!(vector);
                vector = Some((meta_path_span, meta.value()?.parse::<Type>()?));
                return Ok(());
            }

            Err(meta.error("the attribute must be one of: `name`, `option`, `default`, `vector`"))
        })?;

        if let (Some((name, _)), Some((vector, _))) = (&name, &vector) {
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

        match (&option, &default, &vector) {
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

        Ok(FieldOrArgumentAttribute {
            name,
            option,
            default,
            vector,
        })
    }
}

impl FieldOrArgumentAttribute {
    pub(crate) fn from_attrs(
        attrs: &mut Vec<Attribute>,
    ) -> syn::Result<Option<FieldOrArgumentAttribute>> {
        let mut field_or_argument_attr = None;
        let mut errors = Vec::with_capacity(4);
        let mut already_appeared_di = false;

        attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            if already_appeared_di {
                let err = syn::Error::new(attr.span(), "only one `#[di(..)]` attribute is allowed");
                errors.push(err);
            } else {
                match FieldOrArgumentAttribute::try_from(attr) {
                    Ok(o) => field_or_argument_attr = Some(o),
                    Err(e) => errors.push(e),
                }
            }

            already_appeared_di = true;
            false
        });

        if let Some(e) = errors.into_iter().reduce(|mut a, b| {
            a.combine(b);
            a
        }) {
            return Err(e);
        }

        Ok(field_or_argument_attr)
    }

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
