use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    spanned::Spanned, Generics, ImplItem, ImplItemFn, ItemImpl, Path, ReturnType, Type, TypePath,
};

use crate::{
    attr,
    struct_or_function_attribute::{SimpleStructOrFunctionAttribute, StructOrFunctionAttribute},
    utils::{self, Color, Scope},
};

// struct A {
//     a: i32,
// }

// #[Singleton]
// impl A {
//     fn new(#[di(name = "hello")] a:i32) -> Self {
//         Self { a }
//     }
// }

pub(crate) fn generate(
    attribute: StructOrFunctionAttribute,
    mut item_impl: ItemImpl,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let rudi_path = attr::rudi_path(&mut item_impl.attrs)?;

    if let Some(async_constructor) = attribute.async_constructor {
        return Err(syn::Error::new(
            async_constructor.span(),
            "`async_constructor` only support in struct, please use async fn instead",
        ));
    }

    let ItemImpl {
        generics,
        self_ty,
        items,
        trait_,
        ..
    } = &mut item_impl;

    if let Some((_, path, _)) = trait_ {
        return Err(syn::Error::new(
            path.span(),
            "not support impl trait for struct",
        ));
    }

    let simple = attribute.simplify();

    let mut impl_item_fns = items
        .iter_mut()
        .filter_map(|impl_item| match impl_item {
            ImplItem::Fn(impl_item_fn) => Some(impl_item_fn),
            _ => None,
        })
        .collect::<Vec<_>>();

    let default_provider_impl = if impl_item_fns.len() == 1 {
        generate_default_provider_impl(
            rudi_path,
            impl_item_fns.pop().unwrap(),
            self_ty,
            generics,
            &simple,
            scope,
        )?
    } else {
        return Err(syn::Error::new(
            self_ty.span(),
            "must have only one associated function",
        ));
    };

    let expand = quote! {
        #item_impl

        #default_provider_impl
    };

    Ok(expand)
}

fn generate_default_provider_impl(
    rudi_path: Path,
    impl_item_fn: &mut ImplItemFn,
    struct_type_with_generics: &Type,
    struct_generics: &Generics,
    attribute: &SimpleStructOrFunctionAttribute,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let SimpleStructOrFunctionAttribute {
        name,
        eager_create,
        binds,
        async_constructor: _,
        not_auto_register,
    } = attribute;

    #[cfg(feature = "auto-register")]
    utils::check_auto_register_with_generics(*not_auto_register, struct_generics, "struct", scope)?;

    let (return_type_eq_struct_type, return_type_eq_self_type) = match &impl_item_fn.sig.output {
        ReturnType::Type(_, fn_return_type) => {
            let return_type_eq_struct_type = &**fn_return_type == struct_type_with_generics;

            let return_type_eq_self_type = match &**fn_return_type {
                Type::Path(TypePath {
                    qself,
                    path:
                        Path {
                            leading_colon,
                            segments,
                        },
                }) => {
                    qself.is_none()
                        && leading_colon.is_none()
                        && segments.len() == 1
                        && segments.first().unwrap().ident == "Self"
                }
                _ => false,
            };

            (return_type_eq_struct_type, return_type_eq_self_type)
        }
        _ => (false, false),
    };

    if !return_type_eq_struct_type && !return_type_eq_self_type {
        return Err(syn::Error::new(
            impl_item_fn.sig.span(),
            format!(
                "return type must be `{}` or `Self`",
                struct_type_with_generics.into_token_stream()
            ),
        ));
    }

    let color = match impl_item_fn.sig.asyncness {
        Some(_) => Color::Async,
        None => Color::Sync,
    };

    let args = utils::generate_arguments_resolve_methods(&mut impl_item_fn.sig.inputs, color)?;

    let create_provider = utils::generate_create_provider(scope, color);

    let (impl_generics, _, where_clause) = struct_generics.split_for_impl();

    let fn_ident = &impl_item_fn.sig.ident;

    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                    Self::#fn_ident(#(#args),*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| Self::#fn_ident(#(#args),*)
            }
        }
    };

    let auto_register = if *not_auto_register {
        quote! {}
    } else {
        #[cfg(feature = "auto-register")]
        quote! {
            #rudi_path::register_provider!(<#struct_type_with_generics as #rudi_path::DefaultProvider>::provider());
        }
        #[cfg(not(feature = "auto-register"))]
        quote! {}
    };

    let expand = quote! {
        impl #impl_generics #rudi_path::DefaultProvider for #struct_type_with_generics #where_clause {
            type Type = Self;

            fn provider() -> #rudi_path::Provider<Self> {
                <#rudi_path::Provider<_> as ::core::convert::From<_>>::from(
                    #rudi_path::#create_provider(#constructor)
                        .name(#name)
                        .eager_create(#eager_create)
                        #binds
                )
            }
        }

        #auto_register
    };

    Ok(expand)
}
