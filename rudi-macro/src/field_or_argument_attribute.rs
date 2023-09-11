use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{meta::ParseNestedMeta, parse_quote, spanned::Spanned, Attribute, Expr, Token, Type};

// #[di(
//     name = "..",
//     option,
//     default = 42,
//     vec,
//     ref = T
// )]

#[derive(Default)]
pub(crate) struct FieldOrArgumentAttribute {
    name: Option<(Span, Expr)>,
    option: Option<Span>,
    default: Option<(Span, Expr)>,
    vec: Option<Span>,
    pub(crate) ref_: Option<(Span, Option<Type>)>,
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

        if meta_path.is_ident("ref") {
            if self.ref_.is_some() {
                return Err(meta.error("duplicate `ref` argument"));
            }

            self.ref_ = Some((
                meta_path_span,
                if meta.input.is_empty() || meta.input.peek(Token![,]) {
                    None
                } else {
                    Some(meta.value()?.parse::<Type>()?)
                },
            ));
            return Ok(());
        }

        Err(meta.error("the argument must be one of: `name`, `option`, `default`, `vec`, `ref`"))
    }

    fn parse_attr(&mut self, attr: &Attribute) -> syn::Result<()> {
        attr.parse_nested_meta(|meta| self.parse(meta))?;

        let FieldOrArgumentAttribute {
            name,
            option,
            default,
            vec,
            ref_: _ref_,
        } = &self;

        if let (Some((name, _)), Some(vec)) = (name, vec) {
            macro_rules! err {
                ($span:expr) => {
                    syn::Error::new(
                        *$span,
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
                            *$span,
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
                            *$span,
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
                            *$span,
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
                            *$span,
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

        Ok(())
    }

    pub(crate) fn from_attrs(attrs: &mut Vec<Attribute>) -> syn::Result<FieldOrArgumentAttribute> {
        let mut field_or_argument_attr = FieldOrArgumentAttribute::default();
        let mut errors = Vec::new();

        attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            if let Err(e) = field_or_argument_attr.parse_attr(attr) {
                errors.push(e)
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
            ref_,
        } = self;

        SimpleFieldOrArgumentAttribute {
            name: name
                .map(|(_, expr)| quote!(#expr))
                .unwrap_or_else(|| quote!("")),
            option: option.is_some(),
            default: default.map(|(_, expr)| expr),
            vec: vec.is_some(),
            ref_: ref_.map(|(_, ty)| ty),
        }
    }
}

pub(crate) struct SimpleFieldOrArgumentAttribute {
    pub(crate) name: TokenStream,
    pub(crate) option: bool,
    pub(crate) default: Option<Expr>,
    pub(crate) vec: bool,
    pub(crate) ref_: Option<Option<Type>>,
}
