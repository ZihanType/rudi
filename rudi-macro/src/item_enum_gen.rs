use proc_macro2::TokenStream;
use quote::quote;
use rudi_core::{Color, Scope};
use syn::{spanned::Spanned, ItemEnum};

use crate::{
    attr_spans_value::AttrSpansValue,
    commons::{self, FieldResolveStmts, ResolvedFields},
    impl_fn_or_enum_variant_attr::ImplFnOrEnumVariantAttr,
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
        #[cfg(feature = "auto-register")]
        auto_register,
    } = attr.simplify();

    #[cfg(feature = "auto-register")]
    commons::check_generics_when_enable_auto_register(
        auto_register,
        &item_enum.generics,
        commons::ItemKind::Enum,
        scope,
    )?;

    let color = if async_ { Color::Async } else { Color::Sync };

    let mut variant_spans = Vec::new();

    let mut parse_errors = Vec::new();
    let mut duplicate_errors = Vec::new();
    let mut no_matched_variant_errors = Vec::new();

    let matched = item_enum
        .variants
        .iter_mut()
        .filter_map(|variant| {
            variant_spans.push(variant.span());

            match ImplFnOrEnumVariantAttr::parse_attrs(&mut variant.attrs) {
                Ok(None) => None,
                Ok(Some(AttrSpansValue {
                    attr_spans,
                    value: _,
                })) => Some((variant, attr_spans)),
                Err(AttrSpansValue {
                    attr_spans,
                    value: e,
                }) => {
                    parse_errors.push(e);
                    Some((variant, attr_spans))
                }
            }
        })
        .reduce(|first, (_, attr_spans)| {
            attr_spans.into_iter().for_each(|span| {
                let err = syn::Error::new(span, "duplicate `#[di]` attribute");
                duplicate_errors.push(err);
            });

            first
        });

    if matched.is_none() {
        variant_spans.iter().for_each(|span| {
            no_matched_variant_errors.push(syn::Error::new(
                *span,
                "there must be a variant annotated by `#[di]`",
            ));
        });
    }

    if let Some(e) = parse_errors
        .into_iter()
        .chain(duplicate_errors)
        .chain(no_matched_variant_errors)
        .reduce(|mut a, b| {
            a.combine(b);
            a
        })
    {
        return Err(e);
    }

    let (variant, _) = matched.unwrap();

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

    #[cfg(not(feature = "auto-register"))]
    let auto_register = quote! {};

    #[cfg(feature = "auto-register")]
    let auto_register = if auto_register {
        quote! {
            #rudi_path::register_provider!(<#enum_ident as #rudi_path::DefaultProvider>::provider());
        }
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
