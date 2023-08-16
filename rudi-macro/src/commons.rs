use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    punctuated::Punctuated, spanned::Spanned, Attribute, Field, Fields, FieldsNamed, FieldsUnnamed,
    FnArg, Ident, PatType, Token,
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
    let attr = match FieldOrArgumentAttribute::from_attrs(attrs)? {
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
    } = attr.simplify();

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

pub(crate) fn generate_argument_resolve_methods(
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
pub(crate) enum ItemKind {
    Struct,
    Enum,
    Function,

    // impl block
    StructOrEnum,
}

#[cfg(feature = "auto-register")]
impl ItemKind {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ItemKind::Struct => "struct",
            ItemKind::Enum => "enum",
            ItemKind::Function => "function",
            ItemKind::StructOrEnum => "struct or enum",
        }
    }
}

#[cfg(feature = "auto-register")]
pub(crate) fn check_auto_register_with_generics(
    auto_register: bool,
    generics: &syn::Generics,
    item_kind: ItemKind,
    scope: Scope,
) -> syn::Result<()> {
    if auto_register && !generics.params.is_empty() {
        return Err(syn::Error::new(
            generics.span(),
            format!(
                "not support auto register generics {}, \
                please remove generics, or use `#[{}(auto_register = false)]` to disable auto register",
                item_kind.as_str(),
                scope.as_str()
            ),
        ));
    }

    Ok(())
}

pub(crate) enum FieldResolveMethods {
    Unit,
    Named(Vec<Ident>, Vec<TokenStream>),
    Unnamed(Vec<TokenStream>),
}

pub(crate) fn generate_field_resolve_methods(
    fields: &mut Fields,
    color: Color,
) -> syn::Result<FieldResolveMethods> {
    match fields {
        Fields::Unit => Ok(FieldResolveMethods::Unit),
        Fields::Named(FieldsNamed { named, .. }) => {
            let len = named.len();
            let mut idents = Vec::with_capacity(len);
            let mut resolve_methods = Vec::with_capacity(len);

            for Field { attrs, ident, .. } in named {
                resolve_methods.push(generate_only_one_field_or_argument_resolve_method(
                    attrs, color,
                )?);
                idents.push(ident.clone().unwrap());
            }

            Ok(FieldResolveMethods::Named(idents, resolve_methods))
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let mut resolve_methods = Vec::with_capacity(unnamed.len());

            for Field { attrs, .. } in unnamed {
                resolve_methods.push(generate_only_one_field_or_argument_resolve_method(
                    attrs, color,
                )?);
            }

            Ok(FieldResolveMethods::Unnamed(resolve_methods))
        }
    }
}
