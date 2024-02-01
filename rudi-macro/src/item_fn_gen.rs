use from_attr::{AttrsValue, FlagOrValue, FromAttr};
use proc_macro2::TokenStream;
use quote::quote;
use rudi_core::{Color, Scope};
use syn::{GenericParam, ItemFn, ReturnType};

use crate::{
    commons::{self, ArgumentResolveStmts},
    di_attr::DiAttr,
    struct_or_function_attr::{ClosureOrPath, StructOrFunctionAttr},
};

// #[Singleton]
// fn One(#[di(name = "hello")] i: i32) -> String {
//     i.to_string()
// }

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    mut item_fn: ItemFn,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let DiAttr { rudi_path } = match DiAttr::remove_attributes(&mut item_fn.attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => DiAttr::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    match attr.async_ {
        FlagOrValue::Flag { path } | FlagOrValue::Value { path, .. } => {
            return Err(syn::Error::new(
                path,
                "`async` only support in struct and enum, please use async fn or sync fn instead",
            ));
        }
        FlagOrValue::None => {}
    }

    let StructOrFunctionAttr {
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
        auto_register,
        &item_fn.sig.generics,
        commons::ItemKind::Function,
        scope,
    )?;

    let color = match item_fn.sig.asyncness {
        Some(_) => Color::Async,
        None => Color::Sync,
    };

    let condition = condition
        .map(|ClosureOrPath(expr)| quote!(Some(#expr)))
        .unwrap_or_else(|| quote!(None));

    let ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    } = commons::generate_argument_resolve_methods(&mut item_fn.sig.inputs, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

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
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #ident #turbofish (#(#args,)*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #ident #turbofish (#(#args,)*)
                }
            }
        }
    };

    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};

    #[cfg(feature = "auto-register")]
    let auto_register = if auto_register {
        quote! {
            #rudi_path::register_provider!(<#ident as #rudi_path::DefaultProvider>::provider());
        }
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
                        .condition(#condition)
                        #(
                            .bind(#binds)
                        )*
                )
            }
        }

        #auto_register
    };

    Ok(expand)
}
