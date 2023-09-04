use syn::{parse_quote, Attribute, Path};

// #[di(rudi_path = path::to::rudi)]
pub(crate) fn rudi_path(attrs: &mut Vec<Attribute>) -> syn::Result<Path> {
    let mut rudi_path = None;
    let mut errors = Vec::new();

    attrs.retain(|attr| {
        if !attr.path().is_ident("di") {
            return true;
        }

        if let Err(err) = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("rudi_path") {
                if rudi_path.is_some() {
                    return Err(meta.error("duplicate `rudi_path` argument"));
                }
                let path = meta.value()?.call(Path::parse_mod_style)?;
                rudi_path = Some(path);
                Ok(())
            } else {
                Err(meta.error("the argument must be `rudi_path`"))
            }
        }) {
            errors.push(err);
        }

        false
    });

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    Ok(rudi_path.unwrap_or_else(|| parse_quote!(::rudi)))
}
