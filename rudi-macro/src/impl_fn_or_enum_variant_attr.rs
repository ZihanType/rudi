use syn::{spanned::Spanned, Attribute};

use crate::attr_spans_value::AttrSpansValue;

pub(crate) struct ImplFnOrEnumVariantAttr;

impl ImplFnOrEnumVariantAttr {
    pub(crate) fn parse_attrs(
        attrs: &mut Vec<Attribute>,
    ) -> Result<Option<AttrSpansValue<Self>>, AttrSpansValue<syn::Error>> {
        let mut errors = Vec::new();
        let mut attr_spans = Vec::new();

        attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            attr_spans.push(attr.span());

            if let Err(e) = attr.meta.require_path_only() {
                errors.push(e);
            }

            false
        });

        if attr_spans.is_empty() {
            return Ok(None);
        }

        if let Some(e) = errors.into_iter().reduce(|mut a, b| {
            a.combine(b);
            a
        }) {
            return Err(AttrSpansValue {
                attr_spans,
                value: e,
            });
        }

        Ok(Some(AttrSpansValue {
            attr_spans,
            value: Self,
        }))
    }
}
