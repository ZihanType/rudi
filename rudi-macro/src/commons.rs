use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_quote, punctuated::Punctuated, spanned::Spanned, AngleBracketedGenericArguments,
    Attribute, Field, Fields, FieldsNamed, FieldsUnnamed, FnArg, GenericArgument, Ident, PatType,
    Path, PathArguments, PathSegment, Stmt, Token, Type, TypePath, TypeReference,
};

use crate::field_or_argument_attribute::{
    FieldOrArgumentAttribute, SimpleFieldOrArgumentAttribute,
};

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
        create_singleton: Stmt,
        get_singleton: Stmt,
    },
}

struct ResolveOne {
    stmt: ResolveOneValue,
    variable: Ident,
}

fn generate_only_one_field_or_argument_resolve_method(
    attrs: &mut Vec<Attribute>,
    color: Color,
    index: usize,
    field_or_argument_ty: &Type,
    scope: Scope,
) -> syn::Result<ResolveOne> {
    let attr = FieldOrArgumentAttribute::from_attrs(attrs)?;

    match (&attr.ref_, scope) {
        (_, Scope::Singleton) => {}
        (Some((span, _)), /* not singleton */ _) => {
            return Err(syn::Error::new(
                *span,
                "only support `ref` argument in `#[Singleton]` item",
            ))
        }
        _ => {}
    }

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

                let create_singleton = match color {
                    Color::Async => parse_quote! {
                        cx.try_create_singleton_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_create_singleton_with_name::<#ty>(#name);
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = cx.get_singleton_option_with_name(#name);
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
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

                let create_singleton = match color {
                    Color::Async => parse_quote! {
                        cx.try_create_singleton_with_name_async::<#ty>(#name).await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_create_singleton_with_name::<#ty>(#name);
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = match cx.get_singleton_option_with_name(#name) {
                        Some(value) => value,
                        None => #default,
                    };
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
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

                let create_singleton = match color {
                    Color::Async => parse_quote! {
                        cx.try_create_singletons_by_type_async::<#ty>().await;
                    },
                    Color::Sync => parse_quote! {
                        cx.try_create_singletons_by_type::<#ty>();
                    },
                };

                let get_singleton = parse_quote! {
                    let #ident = cx.get_singletons_by_type();
                };

                Ok(ResolveOne {
                    stmt: ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
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

            let create_singleton = match color {
                Color::Async => parse_quote! {
                    cx.just_create_singleton_with_name_async::<#ty>(#name).await;
                },
                Color::Sync => parse_quote! {
                    cx.just_create_singleton_with_name::<#ty>(#name);
                },
            };

            let get_singleton = parse_quote! {
                let #ident = cx.get_singleton_with_name(#name);
            };

            Ok(ResolveOne {
                stmt: ResolveOneValue::Ref {
                    create_singleton,
                    get_singleton,
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
    pub(crate) mut_ref_cx_stmts: Vec<Stmt>,
    pub(crate) ref_cx_stmts: Vec<Stmt>,
    pub(crate) args: Vec<Ident>,
}

pub(crate) fn generate_argument_resolve_methods(
    inputs: &mut Punctuated<FnArg, Token![,]>,
    color: Color,
    scope: Scope,
) -> syn::Result<ArgumentResolveStmts> {
    let capacity = inputs.len();

    let mut mut_ref_cx_stmts = Vec::with_capacity(capacity);
    let mut ref_cx_stmts = Vec::with_capacity(capacity);
    let mut args = Vec::with_capacity(capacity);

    for (index, input) in inputs.iter_mut().enumerate() {
        match input {
            FnArg::Receiver(r) => {
                return Err(syn::Error::new(r.span(), "not support `self` receiver"))
            }
            FnArg::Typed(PatType { attrs, ty, .. }) => {
                let ResolveOne { stmt, variable } =
                    generate_only_one_field_or_argument_resolve_method(
                        attrs, color, index, ty, scope,
                    )?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => mut_ref_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
                    } => {
                        mut_ref_cx_stmts.push(create_singleton);
                        ref_cx_stmts.push(get_singleton);
                    }
                }

                args.push(variable);
            }
        }
    }

    Ok(ArgumentResolveStmts {
        mut_ref_cx_stmts,
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
                please remove generics, or use `#[{}(auto_register = false)]` to disable auto register",
                item_kind.as_str(),
                scope.as_str()
            ),
        ));
    }

    Ok(())
}

pub(crate) struct FieldResolveStmts {
    pub(crate) mut_ref_cx_stmts: Vec<Stmt>,
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

pub(crate) fn generate_field_resolve_methods(
    fields: &mut Fields,
    color: Color,
    scope: Scope,
) -> syn::Result<FieldResolveStmts> {
    match fields {
        Fields::Unit => Ok(FieldResolveStmts {
            mut_ref_cx_stmts: Vec::new(),
            ref_cx_stmts: Vec::new(),
            fields: ResolvedFields::Unit,
        }),
        Fields::Named(FieldsNamed { named, .. }) => {
            let capacity = named.len();

            let mut mut_ref_cx_stmts = Vec::with_capacity(capacity);
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
                } = generate_only_one_field_or_argument_resolve_method(
                    attrs, color, index, ty, scope,
                )?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => mut_ref_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
                    } => {
                        mut_ref_cx_stmts.push(create_singleton);
                        ref_cx_stmts.push(get_singleton);
                    }
                }

                field_values.push(field_value);
                field_names.push(field_name.clone().unwrap());
            }

            Ok(FieldResolveStmts {
                mut_ref_cx_stmts,
                ref_cx_stmts,
                fields: ResolvedFields::Named {
                    field_names,
                    field_values,
                },
            })
        }
        Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => {
            let capacity = unnamed.len();

            let mut mut_ref_cx_stmts = Vec::with_capacity(capacity);
            let mut ref_cx_stmts = Vec::with_capacity(capacity);
            let mut field_values = Vec::with_capacity(capacity);

            for (index, Field { attrs, ty, .. }) in unnamed.into_iter().enumerate() {
                let ResolveOne {
                    stmt,
                    variable: field_value,
                } = generate_only_one_field_or_argument_resolve_method(
                    attrs, color, index, ty, scope,
                )?;

                match stmt {
                    ResolveOneValue::Owned { resolve } => mut_ref_cx_stmts.push(resolve),
                    ResolveOneValue::Ref {
                        create_singleton,
                        get_singleton,
                    } => {
                        mut_ref_cx_stmts.push(create_singleton);
                        ref_cx_stmts.push(get_singleton);
                    }
                }

                field_values.push(field_value);
            }

            Ok(FieldResolveStmts {
                mut_ref_cx_stmts,
                ref_cx_stmts,
                fields: ResolvedFields::Unnamed(field_values),
            })
        }
    }
}
