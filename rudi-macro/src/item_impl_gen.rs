use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use rudi_core::{Color, Scope};
use syn::{
    spanned::Spanned, Generics, ImplItem, ImplItemFn, ItemImpl, Path, ReturnType, Type, TypePath,
};

use crate::{
    commons::{self, ArgumentResolveStmts},
    rudi_path_attribute,
    struct_or_function_attribute::{SimpleStructOrFunctionAttribute, StructOrFunctionAttribute},
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
    attr: StructOrFunctionAttribute,
    mut item_impl: ItemImpl,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let rudi_path = rudi_path_attribute::rudi_path(&mut item_impl.attrs)?;

    if let Some((async_, _)) = attr.async_ {
        return Err(syn::Error::new(
            async_,
            "`async` only support in struct and enum, please use async fn instead",
        ));
    }

    let impl_span = item_impl.span();

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
            "not support impl trait for struct or enum",
        ));
    }

    let simple = attr.simplify();

    let mut errors = Vec::new();
    let mut impl_item_fn = None;

    items.iter_mut().for_each(|impl_item| {
        let ImplItem::Fn(f) = impl_item else {
            return;
        };

        if impl_item_fn.is_some() {
            let err = syn::Error::new(f.span(), "duplicate associated function");
            errors.push(err);
        } else {
            impl_item_fn = Some(f);
        }
    });

    let default_provider_impl = match impl_item_fn {
        None => {
            return Err(syn::Error::new(
                impl_span.span(),
                "there must be an associated function",
            ))
        }
        Some(f) => {
            if let Some(e) = errors.into_iter().reduce(|mut a, b| {
                a.combine(b);
                a
            }) {
                return Err(e);
            }

            generate_default_provider_impl(f, self_ty, generics, &simple, scope, rudi_path)?
        }
    };

    let expand = quote! {
        #item_impl

        #default_provider_impl
    };

    Ok(expand)
}

fn generate_default_provider_impl(
    impl_item_fn: &mut ImplItemFn,
    struct_type_with_generics: &Type,
    struct_generics: &Generics,
    attr: &SimpleStructOrFunctionAttribute,
    scope: Scope,
    rudi_path: Path,
) -> syn::Result<TokenStream> {
    let SimpleStructOrFunctionAttribute {
        name,
        eager_create,
        condition,
        binds,
        async_: _,
        #[cfg(feature = "auto-register")]
        auto_register,
    } = attr;

    #[cfg(feature = "auto-register")]
    commons::check_generics_when_enable_auto_register(
        *auto_register,
        struct_generics,
        commons::ItemKind::StructOrEnum,
        scope,
    )?;

    let (return_type_eq_struct_type, return_type_eq_self_type) = match &impl_item_fn.sig.output {
        ReturnType::Type(_, fn_return_type) => {
            let return_type_eq_struct_type = &**fn_return_type == struct_type_with_generics;

            let return_type_eq_self_type = if let Type::Path(TypePath {
                qself: None,
                path:
                    Path {
                        leading_colon: None,
                        segments,
                    },
            }) = &**fn_return_type
            {
                segments.len() == 1 && segments.first().unwrap().ident == "Self"
            } else {
                false
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

    let ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    } = commons::generate_argument_resolve_methods(&mut impl_item_fn.sig.inputs, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let (impl_generics, _, where_clause) = struct_generics.split_for_impl();

    let fn_ident = &impl_item_fn.sig.ident;

    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    Self::#fn_ident(#(#args,)*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    Self::#fn_ident(#(#args,)*)
                }
            }
        }
    };

    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};

    #[cfg(feature = "auto-register")]
    let auto_register = if *auto_register {
        quote! {
            #rudi_path::register_provider!(<#struct_type_with_generics as #rudi_path::DefaultProvider>::provider());
        }
    } else {
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
                        .condition(#condition)
                        #binds
                )
            }
        }

        #auto_register
    };

    Ok(expand)
}
