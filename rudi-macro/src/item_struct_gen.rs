use proc_macro2::TokenStream;
use quote::quote;
use syn::ItemStruct;

use crate::{
    commons::{self, Color, FieldResolveStmts, ResolvedFields, Scope},
    rudi_path_attribute,
    struct_or_function_attribute::{SimpleStructOrFunctionAttribute, StructOrFunctionAttribute},
};

pub(crate) fn generate(
    attr: StructOrFunctionAttribute,
    mut item_struct: ItemStruct,
    scope: Scope,
) -> syn::Result<TokenStream> {
    let rudi_path = rudi_path_attribute::rudi_path(&mut item_struct.attrs)?;

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
        &item_struct.generics,
        commons::ItemKind::Struct,
        scope,
    )?;

    let color = if async_ { Color::Async } else { Color::Sync };

    let FieldResolveStmts {
        mut_ref_cx_stmts,
        ref_cx_stmts,
        fields,
    } = commons::generate_field_resolve_methods(&mut item_struct.fields, color, scope)?;

    let create_provider = commons::generate_create_provider(scope, color);

    let struct_ident = &item_struct.ident;

    let (impl_generics, ty_generics, where_clause) = item_struct.generics.split_for_impl();

    let instance = match fields {
        ResolvedFields::Unit => quote! {
            #struct_ident
        },
        ResolvedFields::Named {
            field_names,
            field_values,
        } => {
            quote! {
                #struct_ident {
                    #(
                        #field_names: #field_values,
                    )*
                }
            }
        }
        ResolvedFields::Unnamed(field_values) => {
            quote! {
                #struct_ident(
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
                    #(#mut_ref_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #instance
                })
            }
        }
        Color::Sync => {
            quote! {
                #[allow(unused_variables)]
                |cx| {
                    #(#mut_ref_cx_stmts)*
                    #(#ref_cx_stmts)*
                    #instance
                }
            }
        }
    };

    let auto_register = if auto_register {
        #[cfg(feature = "auto-register")]
        quote! {
            #rudi_path::register_provider!(<#struct_ident as #rudi_path::DefaultProvider>::provider());
        }
        #[cfg(not(feature = "auto-register"))]
        quote! {}
    } else {
        quote! {}
    };

    let expand = quote! {
        #item_struct

        impl #impl_generics #rudi_path::DefaultProvider for #struct_ident #ty_generics #where_clause {
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
