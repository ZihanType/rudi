use from_attr::{ConvertParsed, FlagOrValue, FromAttr, PathValue};
use syn::{parse_quote, spanned::Spanned, Expr, ExprPath};

#[derive(FromAttr)]
#[attribute(idents = [di])]
pub(crate) struct StructOrFunctionAttribute {
    #[attribute(default = default_name())]
    pub(crate) name: Expr,

    pub(crate) eager_create: bool,

    pub(crate) condition: Option<ClosureOrPath>,

    pub(crate) binds: Vec<ExprPath>,

    #[attribute(rename = "async")]
    pub(crate) async_: FlagOrValue<bool>,

    #[cfg(feature = "auto-register")]
    #[attribute(default = DEFAULT_AUTO_REGISTER)]
    pub(crate) auto_register: bool,
}

fn default_name() -> Expr {
    parse_quote!("")
}

#[cfg(feature = "auto-register")]
const DEFAULT_AUTO_REGISTER: bool = true;

pub(crate) struct ClosureOrPath(pub(crate) Expr);

impl ConvertParsed for ClosureOrPath {
    type Type = Expr;

    fn convert(path_value: PathValue<Self::Type>) -> syn::Result<Self> {
        let expr = path_value.value;

        match &expr {
            Expr::Closure(_) | Expr::Path(_) => Ok(Self(expr)),
            _ => Err(syn::Error::new(
                expr.span(),
                "the expr must be a closure or an expression path",
            )),
        }
    }
}
