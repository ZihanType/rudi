use proc_macro2::TokenStream;
use quote::quote;
use rudi_core::{Color, Scope};
use syn::{spanned::Spanned, ItemEnum};

use crate::{
    commons::{self, FieldResolveStmts, ResolvedFields},
    rudi_path_attribute,
    struct_or_function_attribute::{SimpleStructOrFunctionAttribute, StructOrFunctionAttribute},
};

pub(crate) fn generate(
    attr: StructOrFunctionAttribute,
    mut item_enum: ItemEnum,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let rudi_path = rudi_path_attribute::rudi_path(&mut item_enum.attrs)?;

    if item_enum.variants.is_empty() {
        return Err(syn::Error::new(item_enum.span(), "not support empty enum"));
    }

    let SimpleStructOrFunctionAttribute {
        name,
        eager_create,
        condition,
        binds,
        async_,
        auto_register,
    } = attr.simplify();

    #[cfg(feature = "auto-register")]
    commons::check_auto_register_with_generics(
        auto_register,
        &item_enum.generics,
        commons::ItemKind::Enum,
        scope,
    )?;

    let color = if async_ { Color::Async } else { Color::Sync };

    let mut annotated_di_variant = None;
    let mut errors = Vec::new();
    let mut di_already_appeared = false;
    let mut variant_spans = Vec::new();

    item_enum.variants.iter_mut().for_each(|variant| {
        variant_spans.push(variant.span());

        variant.attrs.retain(|attr| {
            if !attr.path().is_ident("di") {
                return true;
            }

            if di_already_appeared {
                let err = syn::Error::new(attr.span(), "duplicate `#[di]` attribute");
                errors.push(err);
            } else {
                di_already_appeared = true;

                if let Err(e) = attr.meta.require_path_only() {
                    errors.push(e);
                }
            }

            false
        });

        if annotated_di_variant.is_none() && di_already_appeared {
            annotated_di_variant = Some(variant);
        }
    });

    if annotated_di_variant.is_none() {
        variant_spans.into_iter().for_each(|span| {
            errors.push(syn::Error::new(
                span,
                "there must be a variant annotated by `#[di]`",
            ));
        });
    }

    if let Some(e) = errors.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        return Err(e);
    }

    let variant = annotated_di_variant.unwrap();

    let FieldResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        fields,
    } = commons::generate_field_resolve_stmts(&mut variant.fields, color)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let enum_ident = &item_enum.ident;
    let variant_ident = &variant.ident;

    let (impl_generics, ty_generics, where_clause) = item_enum.generics.split_for_impl();

    let instance = match fields {
        ResolvedFields::Unit => quote! {
            #variant_ident
        },
        ResolvedFields::Named {
            field_names,
            field_values,
        } => {
            quote! {
                #variant_ident {
                    #(
                        #field_names: #field_values,
                    )*
                }
            }
        }
        ResolvedFields::Unnamed(field_values) => {
            quote! {
                #variant_ident(
                    #(
                        #field_values,
                    )*
                )
            }
        }
    };

    let constructor = match color {
        Color::Async => {
            quote! {
                #[allow(unused_variables)]
                |cx| ::std::boxed::Box::pin(async {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #enum_ident::#instance
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#ref_mut_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #enum_ident::#instance
                }
            }
        }
    };

    let auto_register = if auto_register {
        #[cfg(feature = "auto-register")]
        quote! {
            #rudi_path::register_provider!(<#enum_ident as #rudi_path::DefaultProvider>::provider());
        }
        #[cfg(not(feature = "auto-register"))]
        quote! {}
    } else {
        quote! {}
    };

    let expand = quote! {
        #item_enum

        impl #impl_generics #rudi_path::DefaultProvider for #enum_ident #ty_generics #where_clause {
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
