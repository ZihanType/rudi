use syn::{parse_quote, Attribute, Path};

// #[rudi(crate = path::to::rudi)]
pub(crate) fn rudi_path(attrs: &mut Vec<Attribute>) -> syn::Result<Path> {
    let mut rudi_path = None;
    let mut errors: Option<syn::Error> = None;

    attrs.retain(|attr| {
        if !attr.path().is_ident("rudi") {
            return true;
        }

        if let Err(err) = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("crate") {
                if rudi_path.is_some() {
                    return Err(meta.error("duplicate rudi crate attribute"));
                }
                let path = meta.value()?.call(Path::parse_mod_style)?;
                rudi_path = Some(path);
                Ok(())
            } else {
                Err(meta.error("unsupported rudi attribute"))
            }
        }) {
            match &mut errors {
                None => errors = Some(err),
                Some(errors) => errors.combine(err),
            }
        }
        false
    });

    match errors {
        None => Ok(rudi_path.unwrap_or_else(|| parse_quote!(::rudi))),
        Some(errors) => Err(errors),
    }
}
