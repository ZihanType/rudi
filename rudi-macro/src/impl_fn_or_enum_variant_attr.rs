use from_attr::FromAttr;

#[derive(FromAttr)]
#[attribute(idents = [di])]
pub(crate) struct ImplFnOrEnumVariantAttr;
