use from_attr::FromAttr;
use syn::{parse_quote, Path};

// #[di(rudi_path = path::to::rudi)]

#[derive(FromAttr)]
#[attribute(idents = [di])]
pub(crate) struct DiAttr {
    #[attribute(default = default_rudi_path())]
    pub(crate) rudi_path: Path,
}

fn default_rudi_path() -> Path {
    parse_quote!(::rudi)
}

impl Default for DiAttr {
    fn default() -> Self {
        Self {
            rudi_path: default_rudi_path(),
        }
    }
}
