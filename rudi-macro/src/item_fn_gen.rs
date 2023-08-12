use proc_macro2::TokenStream;
use quote::quote;
use syn::{GenericParam, ItemFn, ReturnType};

use crate::{
    struct_or_function_attributes::{SimpleStructOrFunctionAttributes, StructOrFunctionAttributes},
    utils::{self, Color, Scope},
};

// #[Singleton]
// fn One(#[di(name = "hello")] i: i32) -> String {
//     i.to_string()
// }

pub(crate) fn generate(
    attrs: StructOrFunctionAttributes,
    mut item_fn: ItemFn,
    scope: Scope,
) -> syn::Result<TokenStream> {
    if let Some((async_, _)) = attrs.async_ {
        return Err(syn::Error::new(
            async_,
            "`async` only support in struct, please use async fn instead",
        ));
    }

    let SimpleStructOrFunctionAttributes {
        name,
        eager_create,
        binds,
        async_: _,
        auto_register,
        rudi_path,
    } = attrs.simplify();

    #[cfg(feature = "auto-register")]
    utils::check_auto_register_with_generics(
        auto_register,
        &item_fn.sig.generics,
        "function",
        scope,
    )?;

    let color = match item_fn.sig.asyncness {
        Some(_) => Color::Async,
        None => Color::Sync,
    };

    let args = utils::generate_arguments_resolve_methods(&mut item_fn.sig.inputs, color)?;

    let create_provider = utils::generate_create_provider(scope, color);

    let (impl_generics, ty_generics, where_clause) = item_fn.sig.generics.split_for_impl();

    let vis = &item_fn.vis;

    let docs = item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"));

    let ident = &item_fn.sig.ident;

    let return_type_ident = match &item_fn.sig.output {
        ReturnType::Default => quote! {
            ()
        },
        ReturnType::Type(_, ty) => quote! {
            #ty
        },
    };

    let struct_definition = if item_fn.sig.generics.params.is_empty() {
        quote! {
            #vis struct #ident;
        }
    } else {
        let members = item_fn
            .sig
            .generics
            .params
            .iter()
            .filter_map(|param| match param {
                GenericParam::Type(ty) => Some(ty),
                _ => None,
            })
            .enumerate()
            .map(|(idx, ty)| {
                let ty_ident = &ty.ident;
                let ident = quote::format_ident!("_mark{}", idx);
                quote! { #ident: ::core::marker::PhantomData<#ty_ident> }
            });

        quote! {
            #[derive(Default)]
            #vis struct #ident #ty_generics { #(#members),*}
        }
    };

    let turbofish = ty_generics.as_turbofish();
    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                     #ident #turbofish (#(#args),*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| #ident #turbofish (#(#args),*)
            }
        }
    };

    let auto_register = if auto_register {
        #[cfg(feature = "auto-register")]
        quote! {
            #rudi_path::register_provider!(<#ident as #rudi_path::DefaultProvider>::provider());
        }
        #[cfg(not(feature = "auto-register"))]
        quote! {}
    } else {
        quote! {}
    };

    let expand = quote! {
        #(#docs)*
        #[allow(non_camel_case_types)]
        #struct_definition

        impl #impl_generics #rudi_path::DefaultProvider for #ident #ty_generics #where_clause {
            type Type = #return_type_ident;

            fn provider() -> #rudi_path::Provider<Self::Type> {
                #[allow(non_snake_case, clippy::too_many_arguments)]
                #item_fn

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
