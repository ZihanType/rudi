use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    meta::ParseNestedMeta, spanned::Spanned, Expr, ExprArray, ExprPath, LitBool, Path, Token,
};

#[derive(Default)]
pub(crate) struct StructOrFunctionAttributes {
    name: Option<(Span, Expr)>,
    eager_create: Option<(Span, bool)>,
    binds: Option<(Span, Vec<ExprPath>)>,
    pub(crate) async_: Option<(Span, bool)>,
    auto_register: Option<(Span, bool)>,
    rudi_path: Option<(Span, Path)>,
}

impl StructOrFunctionAttributes {
    pub(crate) fn parse(&mut self, meta: ParseNestedMeta) -> syn::Result<()> {
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

        macro_rules! boolean_attr {
            ($attribute:tt, $variable:tt) => {
                if meta_path.is_ident(stringify!($attribute)) {
                    if self.$variable.is_some() {
                        return Err(meta.error(concat!(
                            "the `",
                            stringify!($attribute),
                            "` attribute can only be set once"
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

        boolean_attr!(eager_create, eager_create);
        boolean_attr!(async, async_);
        boolean_attr!(auto_register, auto_register);

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

        if meta_path.is_ident("rudi_path") {
            check_duplicate!(rudi_path);
            self.rudi_path = Some((meta_path_span, meta.value()?.call(Path::parse_mod_style)?));
            return Ok(());
        }

        Err(meta.error("the attribute must be one of: `name`, `eager_create`, `binds`, `async`, `auto_register`, `rudi_path`"))
    }

    pub(crate) fn simplify(&self) -> SimpleStructOrFunctionAttributes {
        let StructOrFunctionAttributes {
            name,
            eager_create,
            binds,
            async_,
            auto_register,
            rudi_path,
        } = self;

        SimpleStructOrFunctionAttributes {
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
            auto_register: auto_register
                .map(|(_, auto_register)| auto_register)
                .unwrap_or(true),
            rudi_path: rudi_path
                .as_ref()
                .map(|(_, rudi_path)| {
                    quote! {
                        #rudi_path
                    }
                })
                .unwrap_or_else(|| {
                    quote! {
                        ::rudi
                    }
                }),
        }
    }
}

pub(crate) struct SimpleStructOrFunctionAttributes {
    pub(crate) name: TokenStream,
    pub(crate) eager_create: bool,
    pub(crate) binds: TokenStream,
    pub(crate) async_: bool,
    pub(crate) auto_register: bool,
    pub(crate) rudi_path: TokenStream,
}
