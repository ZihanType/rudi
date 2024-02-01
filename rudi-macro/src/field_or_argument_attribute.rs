use from_attr::{FlagOrValue, FromAttr};
use syn::{parse_quote, Expr, Type};

// #[di(
//     name = "..",
//     option,
//     default = 42,
//     vec,
//     ref = T
// )]

#[derive(FromAttr)]
#[attribute(idents = [di])]
pub(crate) struct FieldOrArgumentAttr {
    #[attribute(default = default_name(), conflicts = [vec])]
    pub(crate) name: Expr,

    #[attribute(conflicts = [default, vec])]
    pub(crate) option: bool,

    #[attribute(conflicts = [option, vec])]
    pub(crate) default: FlagOrValue<Expr>,

    #[attribute(conflicts = [name, option, default])]
    pub(crate) vec: bool,

    #[attribute(rename = "ref")]
    pub(crate) ref_: FlagOrValue<Type>,
}

fn default_name() -> Expr {
    parse_quote!("")
}

impl Default for FieldOrArgumentAttr {
    fn default() -> Self {
        Self {
            name: default_name(),
            option: Default::default(),
            default: Default::default(),
            vec: Default::default(),
            ref_: Default::default(),
        }
    }
}
