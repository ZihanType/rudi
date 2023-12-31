use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{meta::ParseNestedMeta, spanned::Spanned, Expr, ExprArray, ExprPath, LitBool, Token};

#[derive(Default)]
pub(crate) struct StructOrFunctionAttribute {
    name: Option<(Span, Expr)>,
    eager_create: Option<(Span, bool)>,
    condition: Option<(Span, Expr)>,
    binds: Option<(Span, Vec<ExprPath>)>,
    pub(crate) async_: Option<(Span, bool)>,
    #[cfg(feature = "auto-register")]
    auto_register: Option<(Span, bool)>,
}

impl StructOrFunctionAttribute {
    pub(crate) fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
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

        macro_rules! boolean_arg {
            ($argument:tt, $variable:tt) => {
                if meta_path.is_ident(stringify!($argument)) {
                    if self.$variable.is_some() {
                        return Err(meta.error(concat!(
                            "duplicate `",
                            stringify!($argument),
                            "` argument"
                        )));
                    }

                    self.$variable = Some((
                        meta_path_span,
                        if meta.input.is_empty() || meta.input.peek(Token![,]) {
                            true
                        } else {
                            meta.value()?.parse::<LitBool>()?.value
                        },
                    ));
                    return Ok(());
                }
            };
        }

        if meta_path.is_ident("name") {
            check_duplicate!(name);
            self.name = Some((meta_path_span, meta.value()?.parse()?));
            return Ok(());
        }

        boolean_arg!(eager_create, eager_create);
        boolean_arg!(async, async_);
        #[cfg(feature = "auto-register")]
        boolean_arg!(auto_register, auto_register);

        if meta_path.is_ident("condition") {
            check_duplicate!(condition);

            let expr = meta.value()?.parse::<Expr>()?;

            match &expr {
                Expr::Closure(_) | Expr::Path(_) => {}
                _ => {
                    return Err(syn::Error::new(
                        expr.span(),
                        "the argument of `condition` must be a closure or an expression path",
                    ));
                }
            }

            self.condition = Some((meta_path_span, expr));
            return Ok(());
        }

        if meta_path.is_ident("binds") {
            check_duplicate!(binds);

            let array = meta.value()?.parse::<ExprArray>()?;

            let mut paths = vec![];

            for expr in array.elems {
                if let Expr::Path(expr_path) = expr {
                    paths.push(expr_path);
                } else {
                    return Err(syn::Error::new(
                        expr.span(),
                        "the element in `binds` must be an expression path",
                    ));
                }
            }

            self.binds = Some((meta_path_span, paths));
            return Ok(());
        }

        #[allow(unused_mut)]
        let mut err_msg = String::from(
            "the argument must be one of: `name`, `eager_create`, `condition`, `binds`, `async`",
        );

        #[cfg(feature = "auto-register")]
        err_msg.push_str(", `auto_register`");

        Err(meta.error(err_msg))
    }

    pub(crate) fn simplify(&self) -> SimpleStructOrFunctionAttribute {
        let StructOrFunctionAttribute {
            name,
            eager_create,
            condition,
            binds,
            async_,
            #[cfg(feature = "auto-register")]
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
                .unwrap_or_else(|| {
                    quote! {
                        ""
                    }
                }),
            eager_create: eager_create
                .map(|(_, eager_create)| eager_create)
                .unwrap_or(false),
            condition: condition
                .as_ref()
                .map(|(_, condition)| {
                    quote! {
                        Some(#condition)
                    }
                })
                .unwrap_or_else(|| {
                    quote! {
                        None
                    }
                }),
            binds: binds
                .as_ref()
                .map(|(_, binds)| {
                    quote! {
                        #(
                            .bind(#binds)
                        )*
                    }
                })
                .unwrap_or_else(|| quote! {}),
            async_: async_.map(|(_, async_)| async_).unwrap_or(false),
            #[cfg(feature = "auto-register")]
            auto_register: auto_register
                .map(|(_, auto_register)| auto_register)
                .unwrap_or(true),
        }
    }
}

pub(crate) struct SimpleStructOrFunctionAttribute {
    pub(crate) name: TokenStream,
    pub(crate) eager_create: bool,
    pub(crate) condition: TokenStream,
    pub(crate) binds: TokenStream,
    pub(crate) async_: bool,
    #[cfg(feature = "auto-register")]
    pub(crate) auto_register: bool,
}
