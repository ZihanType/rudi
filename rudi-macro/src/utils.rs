use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned, Attribute, FnArg, PatType, Token};

use crate::name::Name;

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

pub(crate) fn get_create_provider(scope: Scope, color: Color) -> TokenStream {
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

pub(crate) fn get_one_arg_or_field_resolve_expr(
    attrs: &mut Vec<Attribute>,
    color: Color,
) -> syn::Result<TokenStream> {
    let mut attrs = drain_filter(attrs, |attr| attr.path().is_ident("di"));
    if attrs.len() > 1 {
        return Err(syn::Error::new(
            attrs[1].span(),
            "only one `#[di(..)]` macro is allowed",
        ));
    }

    let name = match attrs.pop() {
        Some(attr) => Some(attr.parse_args::<Name>()?),
        _ => None,
    };

    let invoke_resolve = match (name, color) {
        (None, Color::Async) => quote! {
            cx.resolve_with_name_async("").await
        },
        (None, Color::Sync) => quote! {
            cx.resolve_with_name("")
        },
        (Some(name), Color::Async) => quote! {
            cx.resolve_with_name_async(#name).await
        },
        (Some(name), Color::Sync) => quote! {
            cx.resolve_with_name(#name)
        },
    };

    Ok(invoke_resolve)
}

pub(crate) fn get_args_resolve_expr(
    args: &mut Punctuated<FnArg, Token![,]>,
    color: Color,
) -> syn::Result<Vec<TokenStream>> {
    let mut ret = Vec::new();

    for input in args.iter_mut() {
        match input {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(r.span(), "not support `self` receiver"))
            }
            FnArg::Typed(PatType { attrs, .. }) => {
                ret.push(get_one_arg_or_field_resolve_expr(attrs, color)?);
            }
        }
    }

    Ok(ret)
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

pub(crate) fn drain_filter<T, F>(vec: &mut Vec<T>, mut predicate: F) -> Vec<T>
where
    F: FnMut(&mut T) -> bool,
{
    let mut ret = Vec::new();
    let mut i = 0;
    while i < vec.len() {
        if predicate(&mut vec[i]) {
            ret.push(vec.remove(i));
        } else {
            i += 1;
        }
    }
    ret
}
