use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{meta::ParseNestedMeta, parse_quote, spanned::Spanned, Attribute, Expr, Token};

// #[di(
//     name = "..",
//     option,
//     default = 42,
//     vec,
// )]

#[derive(Default)]
pub(crate) struct FieldOrArgumentAttribute {
    name: Option<(Span, Expr)>,
    option: Option<Span>,
    default: Option<(Span, Expr)>,
    vec: Option<Span>,
}

impl FieldOrArgumentAttribute {
    fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
        let meta_path = &meta.path;
        let meta_path_span = meta_path.span();

        macro_rules! check_duplicate {
            ($argument:tt) => {
                if self.$argument.is_some() {
                    return Err(meta.error(concat!(
                        "duplicate `",
                        stringify!($argument),
                        "` argument"
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
            self.option = Some(meta_path_span);
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

        if meta_path.is_ident("vec") {
            check_duplicate!(vec);
            self.vec = Some(meta_path_span);
            return Ok(());
        }

        Err(meta.error("the argument must be one of: `name`, `option`, `default`, `vec`"))
    }
}

impl TryFrom<&Attribute> for FieldOrArgumentAttribute {
    type Error = syn::Error;

    fn try_from(attr: &Attribute) -> Result<Self, Self::Error> {
        let mut field_or_argument_attr = FieldOrArgumentAttribute::default();
        attr.parse_nested_meta(|meta| field_or_argument_attr.parse(meta))?;

        let FieldOrArgumentAttribute {
            name,
            option,
            default,
            vec,
        } = &field_or_argument_attr;

        if let (Some((name, _)), Some(vec)) = (name, vec) {
            macro_rules! err {
                ($span:expr) => {
                    syn::Error::new(
                        $span.clone(),
                        "the `name` and `vec` arguments cannot be used together",
                    )
                };
            }

            let mut e = err!(name);
            e.combine(err!(vec));

            return Err(e);
        }

        match (option, default, vec) {
            (Some(option), Some((default, _)), Some(vec)) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option`, `default`, and `vec` arguments cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(default));
                e.combine(err!(vec));

                return Err(e);
            }
            (Some(option), Some((default, _)), None) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option` and `default` arguments cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(default));

                return Err(e);
            }
            (Some(option), None, Some(vec)) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `option` and `vec` arguments cannot be used together",
                        )
                    };
                }

                let mut e = err!(option);
                e.combine(err!(vec));

                return Err(e);
            }
            (None, Some((default, _)), Some(vec)) => {
                macro_rules! err {
                    ($span:expr) => {
                        syn::Error::new(
                            $span.clone(),
                            "the `default` and `vec` arguments cannot be used together",
                        )
                    };
                }

                let mut e = err!(default);
                e.combine(err!(vec));

                return Err(e);
            }
            _ => {}
        }

        Ok(field_or_argument_attr)
    }
}

impl FieldOrArgumentAttribute {
    pub(crate) fn from_attrs(
        attrs: &mut Vec<Attribute>,
    ) -> syn::Result<Option<FieldOrArgumentAttribute>> {
        let mut field_or_argument_attr = None;
        let mut errors = Vec::new();
        let mut di_already_appeared = false;

        attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            if di_already_appeared {
                let err = syn::Error::new(attr.span(), "duplicate `#[di(...)]` attribute");
                errors.push(err);
            } else {
                di_already_appeared = true;

                match FieldOrArgumentAttribute::try_from(attr) {
                    Ok(o) => field_or_argument_attr = Some(o),
                    Err(e) => errors.push(e),
                }
            }

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
            vec,
        } = self;

        SimpleFieldOrArgumentAttribute {
            name: name
                .map(|(_, expr)| quote!(#expr))
                .unwrap_or_else(|| quote!("")),
            option: option.is_some(),
            default: default.map(|(_, expr)| expr),
            vec: vec.is_some(),
        }
    }
}

pub(crate) struct SimpleFieldOrArgumentAttribute {
    pub(crate) name: TokenStream,
    pub(crate) option: bool,
    pub(crate) default: Option<Expr>,
    pub(crate) vec: bool,
}
