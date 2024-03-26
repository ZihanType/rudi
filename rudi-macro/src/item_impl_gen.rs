use from_attr::{AttrsValue, FromAttr, PathValue};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use rudi_core::{Color, Scope};
use syn::{
    parse_quote, spanned::Spanned, Generics, ImplItem, ImplItemFn, ItemImpl, Path, ReturnType,
    Type, TypePath,
};

use crate::{
    commons::{self, ArgumentResolveStmts},
    di_attr::DiAttr,
    impl_fn_or_enum_variant_attr::ImplFnOrEnumVariantAttr,
    struct_or_function_attr::{ClosureOrPath, StructOrFunctionAttr},
};

// struct A {
//     a: i32,
// }

// #[Singleton]
// impl A {
//     #[di]
//     fn new(#[di(name = "hello")] a:i32) -> Self {
//         Self { a }
//     }
// }

pub(crate) fn generate(
    attr: StructOrFunctionAttr,
    mut item_impl: ItemImpl,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let DiAttr { rudi_path } = match DiAttr::remove_attributes(&mut item_impl.attrs) {
        Ok(Some(AttrsValue { value: attr, .. })) => attr,
        Ok(None) => DiAttr::default(),
        Err(AttrsValue { value: e, .. }) => return Err(e),
    };

    if let Some(PathValue { path, .. }) = attr.async_ {
        return Err(syn::Error::new(
            path,
            "`async` only support in struct and enum, please use async fn or sync fn instead",
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

    let trait_ = trait_.as_mut().map(|(_, path, _)| path);

    let mut parse_errors = Vec::new();
    let mut duplicate_errors = Vec::new();
    let mut no_matched_fn_errors = Vec::new();

    let matched = items
        .iter_mut()
        .filter_map(|impl_item| {
            let f = match impl_item {
                ImplItem::Fn(f) => f,
                _ => return None,
            };

            match ImplFnOrEnumVariantAttr::remove_attributes(&mut f.attrs) {
                Ok(None) => None,
                Ok(Some(AttrsValue { attrs, .. })) => Some((f, attrs)),
                Err(AttrsValue { attrs, value: e }) => {
                    parse_errors.push(e);
                    Some((f, attrs))
                }
            }
        })
        .reduce(|first, (_, attrs)| {
            attrs.into_iter().for_each(|attr| {
                let err = syn::Error::new(attr.span(), "duplicate `#[di]` attribute");
                duplicate_errors.push(err);
            });

            first
        });

    if matched.is_none() {
        no_matched_fn_errors.push(syn::Error::new(
            impl_span.span(),
            "there must be an associated function annotated by `#[di]`",
        ));
    }

    if let Some(e) = parse_errors
        .into_iter()
        .chain(duplicate_errors)
        .chain(no_matched_fn_errors)
        .reduce(|mut a, b| {
            a.combine(b);
            a
        })
    {
        return Err(e);
    }

    let (f, _) = matched.unwrap();

    let default_provider_impl =
        generate_default_provider_impl(f, &trait_, self_ty, generics, attr, scope, rudi_path)?;

    let expand = quote! {
        #item_impl

        #default_provider_impl
    };

    Ok(expand)
}

fn generate_default_provider_impl<'a>(
    impl_item_fn: &'a mut ImplItemFn,
    trait_: &'a Option<&'a mut Path>,
    type_with_generics: &'a Type,
    generics: &'a Generics,
    attr: StructOrFunctionAttr,
    scope: Scope,
    rudi_path: Path,
) -> syn::Result<TokenStream> {
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
        generics,
        commons::ItemKind::StructOrEnum,
        scope,
    )?;

    let return_type: Type = match &impl_item_fn.sig.output {
        ReturnType::Default => parse_quote!(()),
        ReturnType::Type(_, return_type) => *return_type.clone(),
    };

    let return_type_is_named = &return_type == type_with_generics;

    let return_type_is_self = if let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }) = &return_type
    {
        segments.len() == 1 && segments.first().unwrap().ident == "Self"
    } else {
        false
    };

    if !return_type_is_named && !return_type_is_self {
        return Err(syn::Error::new(
            impl_item_fn.sig.span(),
            format!(
                "return type must be `{}` or `Self`",
                type_with_generics.into_token_stream()
            ),
        ));
    }

    let color = match impl_item_fn.sig.asyncness {
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
    } = commons::generate_argument_resolve_methods(&mut impl_item_fn.sig.inputs, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let (impl_generics, _, where_clause) = generics.split_for_impl();

    let fn_ident = &impl_item_fn.sig.ident;

    let self_path = match trait_ {
        Some(trait_) => quote! { <Self as #trait_> },
        None => quote! { Self },
    };

    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #self_path::#fn_ident(#(#args,)*).await
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #self_path::#fn_ident(#(#args,)*)
                }
            }
        }
    };

    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};

    #[cfg(feature = "auto-register")]
    let auto_register = if auto_register {
        quote! {
            #rudi_path::register_provider!(<#type_with_generics as #rudi_path::DefaultProvider>::provider());
        }
    } else {
        quote! {}
    };

    let expand = quote! {
        impl #impl_generics #rudi_path::DefaultProvider for #type_with_generics #where_clause {
            type Type = Self;

            fn provider() -> #rudi_path::Provider<Self> {
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
