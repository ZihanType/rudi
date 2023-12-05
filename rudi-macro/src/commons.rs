use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use rudi_core::{Color, Scope};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, AngleBracketedGenericArguments,
    Attribute, Field, Fields, FieldsNamed, FieldsUnnamed, FnArg, GenericArgument, Ident, PatType,
    Path, PathArguments, PathSegment, Stmt, Token, Type, TypePath, TypeReference,
};

use crate::field_or_argument_attribute::{
    FieldOrArgumentAttribute, SimpleFieldOrArgumentAttribute,
};

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
        (Scope::SingleOwner, Color::Async) => quote! {
            single_owner_async
        },
        (Scope::SingleOwner, Color::Sync) => quote! {
            single_owner
        },
    }
}

fn extract_ref_type(ty: &Type) -> syn::Result<&Type> {
    fn require_type_ref(ty: &Type) -> Option<&TypeReference> {
        match ty {
            Type::Reference(type_ref) => Some(type_ref),
            _ => None,
        }
    }

    fn get_type_from_ref(
        TypeReference {
            mutability, elem, ..
        }: &TypeReference,
    ) -> syn::Result<&Type> {
        if mutability.is_some() {
            Err(syn::Error::new(
                mutability.span(),
                "not support mutable reference",
            ))
        } else {
            Ok(elem)
        }
    }

    let mut ty: &Type = match require_type_ref(ty) {
        Some(type_ref) => get_type_from_ref(type_ref)?,
        None => {
            return Err(syn::Error::new(
                ty.span(),
                "not support non-reference type, \
        please change to a reference type, \
        or if using a type alias, specify the original type using `#[di(ref = T)]`, \
        where `T` is a non-reference type",
            ))
        }
    };

    loop {
        ty = match require_type_ref(ty) {
            Some(type_ref) => get_type_from_ref(type_ref)?,
            None => break,
        };
    }

    Ok(ty)
}

fn extract_path_type<'a>(ty: &'a Type, ty_name: &str) -> syn::Result<&'a Type> {
    let Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments,
        },
    }) = ty
    else {
        return Err(syn::Error::new(
            ty.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    };

    let Some(segment) = segments.last() else {
        return Err(syn::Error::new(
            ty.span(),
            "not support path type with empty segments",
        ));
    };

    let PathSegment {
        ident,
        arguments: PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }),
    } = segment
    else {
        return Err(syn::Error::new(
            segment.span(),
            "only support angle bracketed generic argument",
        ));
    };

    if ident != ty_name {
        return Err(syn::Error::new(
            ident.span(),
            format!("only support `{}<T>` type", ty_name),
        ));
    }

    let Some(arg) = args.first() else {
        return Err(syn::Error::new(
            segment.span(),
            format!(
                "not support `{}<T>` type with empty generic arguments ",
                ty_name
            ),
        ));
    };

    if args.len() > 1 {
        let msg = format!(
            "only support `{}<T>` type with one generic argument",
            ty_name
        );

        if let Some(e) = args
            .iter()
            .skip(1)
            .map(|arg| syn::Error::new(arg.span(), &msg))
            .reduce(|mut a, b| {
                a.combine(b);
                a
            })
        {
            return Err(e);
        }
    }

    if let GenericArgument::Type(ty) = arg {
        extract_ref_type(ty)
    } else {
        Err(syn::Error::new(
            arg.span(),
            "only support generic argument type",
        ))
    }
}

fn extract_option_type(ty: &Type) -> syn::Result<&Type> {
    extract_path_type(ty, "Option")
}

fn extract_vec_type(ty: &Type) -> syn::Result<&Type> {
    extract_path_type(ty, "Vec")
}

enum ResolveOneValue {
    Owned {
        resolve: Stmt,
    },
    Ref {
        create_single: Stmt,
        get_single: Stmt,
    },
}

struct ResolveOne {
    stmt: ResolveOneValue,
    variable: Ident,
}

fn generate_only_one_field_or_argument_resolve_stmt(
    attrs: &mut Vec<Attribute>,
    color: Color,
    index: usize,
    field_or_argument_ty: &Type,
) -> syn::Result<ResolveOne> {
    let attr = FieldOrArgumentAttribute::from_attrs(attrs)?;

    let SimpleFieldOrArgumentAttribute {
        name,
        option,
        default,
        vec,
        ref_,
    } = attr.simplify();

    let ident = if ref_.is_some() {
        format_ident!("ref_{}", index)
    } else {
        format_ident!("owned_{}", index)
    };

    if option {
        return match ref_ {
            Some(ref_ty) => {
                let ty = if let Some(ty) = ref_ty {
                    quote!(#ty)
                } else {
                    let ty = extract_option_type(field_or_argument_ty)?;
                    quote!(#ty)
                };

                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_single_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_single_with_name::<#ty>(#name);
                    },
                };

                let get_single = parse_quote! {
                    let #ident = cx.get_single_option_with_name(#name);
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = cx.resolve_option_with_name_async(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        let #ident = cx.resolve_option_with_name(#name);
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    if let Some(default) = default {
        return match ref_ {
            Some(ref_ty) => {
                let ty = if let Some(ty) = ref_ty {
                    quote!(#ty)
                } else {
                    let ty = extract_ref_type(field_or_argument_ty)?;
                    quote!(#ty)
                };

                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_single_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_single_with_name::<#ty>(#name);
                    },
                };

                let get_single = parse_quote! {
                    let #ident = match cx.get_single_option_with_name(#name) {
                        Some(value) => value,
                        None => #default,
                    };
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = match cx.resolve_option_with_name_async(#name).await {
                            Some(value) => value,
                            None => #default,
                        };
                    },
                    Color::Sync => parse_quote! {
                        let #ident = match cx.resolve_option_with_name(#name) {
                            Some(value) => value,
                            None => #default,
                        };
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    if vec {
        return match ref_ {
            Some(ref_ty) => {
                let ty = if let Some(ty) = ref_ty {
                    quote!(#ty)
                } else {
                    let ty = extract_vec_type(field_or_argument_ty)?;
                    quote!(#ty)
                };

                let create_single = match color {
                    Color::Async => parse_quote! {
                        cx.try_just_create_singles_by_type_async::<#ty>().await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_just_create_singles_by_type::<#ty>();
                    },
                };

                let get_single = parse_quote! {
                    let #ident = cx.get_singles_by_type();
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    },
                    variable: ident,
                })
            }
            None => {
                let resolve = match color {
                    Color::Async => parse_quote! {
                        let #ident = cx.resolve_by_type_async().await;
                    },
                    Color::Sync => parse_quote! {
                        let #ident = cx.resolve_by_type();
                    },
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Owned { resolve },
                    variable: ident,
                })
            }
        };
    }

    match ref_ {
        Some(ref_ty) => {
            let ty = if let Some(ty) = ref_ty {
                quote!(#ty)
            } else {
                let ty = extract_ref_type(field_or_argument_ty)?;
                quote!(#ty)
            };

            let create_single = match color {
                Color::Async => parse_quote! {
                    cx.just_create_single_with_name_async::<#ty>(#name).await;
                },
                Color::Sync => parse_quote! {
                    cx.just_create_single_with_name::<#ty>(#name);
                },
            };

            let get_single = parse_quote! {
                let #ident = cx.get_single_with_name(#name);
            };

            Ok(ResolveOne {
                stmt: ResolveOneValue::Ref {
                    create_single,
                    get_single,
                },
                variable: ident,
            })
        }
        None => {
            let resolve = match color {
                Color::Async => parse_quote! {
                    let #ident = cx.resolve_with_name_async(#name).await;
                },
                Color::Sync => parse_quote! {
                    let #ident = cx.resolve_with_name(#name);
                },
            };

            Ok(ResolveOne {
                stmt: ResolveOneValue::Owned { resolve },
                variable: ident,
            })
        }
    }
}

pub(crate) struct ArgumentResolveStmts {
    pub(crate) ref_mut_cx_stmts: Vec<Stmt>,
    pub(crate) ref_cx_stmts: Vec<Stmt>,
    pub(crate) args: Vec<Ident>,
}

pub(crate) fn generate_argument_resolve_methods(
    inputs: &mut Punctuated<FnArg, Token![,]>,
    color: Color,
) -> syn::Result<ArgumentResolveStmts> {
    let capacity = inputs.len();

    let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
    let mut ref_cx_stmts = Vec::with_capacity(capacity);
    let mut args = Vec::with_capacity(capacity);

    for (index, input) in inputs.iter_mut().enumerate() {
        match input {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(r.span(), "not support `self` receiver"))
            }
            FnArg::Typed(PatType { attrs, ty, .. }) => {
                let ResolveOne { stmt, variable } =
                    generate_only_one_field_or_argument_resolve_stmt(attrs, color, index, ty)?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => ref_mut_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    } => {
                        ref_mut_cx_stmts.push(create_single);
                        ref_cx_stmts.push(get_single);
                    }
                }

                args.push(variable);
            }
        }
    }

    Ok(ArgumentResolveStmts {
        ref_mut_cx_stmts,
        ref_cx_stmts,
        args,
    })
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
                please remove generics, or use `#[{:?}(auto_register = false)]` to disable auto register",
                item_kind.as_str(),
                scope
            ),
        ));
    }

    Ok(())
}

pub(crate) struct FieldResolveStmts {
    pub(crate) ref_mut_cx_stmts: Vec<Stmt>,
    pub(crate) ref_cx_stmts: Vec<Stmt>,
    pub(crate) fields: ResolvedFields,
}

pub(crate) enum ResolvedFields {
    Unit,
    Named {
        field_names: Vec<Ident>,
        field_values: Vec<Ident>,
    },
    Unnamed(Vec<Ident>),
}

pub(crate) fn generate_field_resolve_stmts(
    fields: &mut Fields,
    color: Color,
) -> syn::Result<FieldResolveStmts> {
    match fields {
        Fields::Unit => Ok(FieldResolveStmts {
            ref_mut_cx_stmts: Vec::new(),
            ref_cx_stmts: Vec::new(),
            fields: ResolvedFields::Unit,
        }),
        Fields::Named(FieldsNamed { named, .. }) => {
            let capacity = named.len();

            let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
            let mut ref_cx_stmts = Vec::with_capacity(capacity);
            let mut field_values = Vec::with_capacity(capacity);

            let mut field_names = Vec::with_capacity(capacity);

            for (
                index,
                Field {
                    attrs,
                    ident: field_name,
                    ty,
                    ..
                },
            ) in named.into_iter().enumerate()
            {
                let ResolveOne {
                    stmt,
                    variable: field_value,
                } = generate_only_one_field_or_argument_resolve_stmt(attrs, color, index, ty)?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => ref_mut_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    } => {
                        ref_mut_cx_stmts.push(create_single);
                        ref_cx_stmts.push(get_single);
                    }
                }

                field_values.push(field_value);
                field_names.push(field_name.clone().unwrap());
            }

            Ok(FieldResolveStmts {
                ref_mut_cx_stmts,
                ref_cx_stmts,
                fields: ResolvedFields::Named {
                    field_names,
                    field_values,
                },
            })
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let capacity = unnamed.len();

            let mut ref_mut_cx_stmts = Vec::with_capacity(capacity);
            let mut ref_cx_stmts = Vec::with_capacity(capacity);
            let mut field_values = Vec::with_capacity(capacity);

            for (index, Field { attrs, ty, .. }) in unnamed.into_iter().enumerate() {
                let ResolveOne {
                    stmt,
                    variable: field_value,
                } = generate_only_one_field_or_argument_resolve_stmt(attrs, color, index, ty)?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => ref_mut_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_single,
                        get_single,
                    } => {
                        ref_mut_cx_stmts.push(create_single);
                        ref_cx_stmts.push(get_single);
                    }
                }

                field_values.push(field_value);
            }

            Ok(FieldResolveStmts {
                ref_mut_cx_stmts,
                ref_cx_stmts,
                fields: ResolvedFields::Unnamed(field_values),
            })
        }
    }
}
