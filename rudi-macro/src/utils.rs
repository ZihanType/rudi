use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, FnArg, Meta, MetaNameValue, PatType, Path,
    Token,
};

use crate::field_or_argument_attribute::{
    FieldOrArgumentAttribute, SimpleFieldOrArgumentAttribute,
};

#[derive(Clone, Copy)]
pub(crate) enum Scope {
    Singleton,
    Transient,
}

#[cfg(feature = "auto-register")]
impl Scope {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Scope::Singleton => "Singleton",
            Scope::Transient => "Transient",
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Color {
    Async,
    Sync,
}

pub(crate) fn generate_create_provider(scope: Scope, color: Color) -> TokenStream {
    match (scope, color) {
        (Scope::Singleton, Color::Async) => quote! {
            singleton_async
        },
        (Scope::Singleton, Color::Sync) => quote! {
            singleton
        },
        (Scope::Transient, Color::Async) => quote! {
            transient_async
        },
        (Scope::Transient, Color::Sync) => quote! {
            transient
        },
    }
}

pub(crate) fn generate_only_one_field_or_argument_resolve_method(
    attrs: &mut Vec<Attribute>,
    color: Color,
) -> syn::Result<TokenStream> {
    let field_or_argument_attr = match FieldOrArgumentAttribute::from_attrs(attrs)? {
        Some(attr) => attr,
        None => {
            return Ok(match color {
                Color::Async => quote! {
                    cx.resolve_with_name_async("").await
                },
                Color::Sync => quote! {
                    cx.resolve_with_name("")
                },
            })
        }
    };

    let SimpleFieldOrArgumentAttribute {
        name,
        option,
        default,
        vector,
    } = field_or_argument_attr.simplify();

    if let Some(ty) = option {
        return Ok(match color {
            Color::Async => quote! {
                cx.resolve_option_with_name_async::<#ty>(#name).await
            },
            Color::Sync => quote! {
                cx.resolve_option_with_name::<#ty>(#name)
            },
        });
    }

    if let Some(default) = default {
        return Ok(match color {
            Color::Async => quote! {
                match cx.resolve_option_with_name_async(#name).await {
                    Some(value) => value,
                    None => #default,
                }
            },
            Color::Sync => quote! {
                match cx.resolve_option_with_name(#name) {
                    Some(value) => value,
                    None => #default,
                }
            },
        });
    }

    if let Some(ty) = vector {
        return Ok(match color {
            Color::Async => quote! {
                cx.resolve_by_type_async::<#ty>().await
            },
            Color::Sync => quote! {
                cx.resolve_by_type::<#ty>()
            },
        });
    }

    Ok(match color {
        Color::Async => quote! {
            cx.resolve_with_name_async(#name).await
        },
        Color::Sync => quote! {
            cx.resolve_with_name(#name)
        },
    })
}

pub(crate) fn generate_arguments_resolve_methods(
    inputs: &mut Punctuated<FnArg, Token![,]>,
    color: Color,
) -> syn::Result<Vec<TokenStream>> {
    let mut args = Vec::new();

    for input in inputs.iter_mut() {
        match input {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(r.span(), "not support `self` receiver"))
            }
            FnArg::Typed(PatType { attrs, .. }) => {
                args.push(generate_only_one_field_or_argument_resolve_method(
                    attrs, color,
                )?);
            }
        }
    }

    Ok(args)
}

#[cfg(feature = "auto-register")]
pub(crate) fn check_auto_register_with_generics(
    not_auto_register: bool,
    generics: &syn::Generics,
    item_type: &'static str,
    scope: Scope,
) -> syn::Result<()> {
    if !not_auto_register && !generics.params.is_empty() {
        return Err(syn::Error::new(
            generics.span(),
            format!(
                "not support auto register generic {} into `AutoRegisterModule`, \
                please remove generics, or use `#[{}(not_auto_register)]` to disable auto register",
                item_type,
                scope.as_str()
            ),
        ));
    }

    Ok(())
}

pub(crate) fn require_path_only(meta: Meta) -> syn::Result<Path> {
    meta.require_path_only()?;

    match meta {
        Meta::Path(path) => Ok(path),
        _ => unreachable!(),
    }
}

pub(crate) fn require_name_value(meta: Meta) -> syn::Result<MetaNameValue> {
    meta.require_name_value()?;

    match meta {
        Meta::NameValue(name_value) => Ok(name_value),
        _ => unreachable!(),
    }
}
