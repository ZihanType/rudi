mod commons;
mod di_attr;
mod field_or_argument_attr;
mod impl_fn_or_enum_variant_attr;
mod item_enum_gen;
mod item_fn_gen;
mod item_impl_gen;
mod item_struct_gen;
mod struct_or_function_attr;

use from_attr::FromAttr;
use proc_macro::TokenStream;
use rudi_core::Scope;
use syn::{parse_macro_input, spanned::Spanned, Item};

use crate::struct_or_function_attr::StructOrFunctionAttr;

fn generate(attr: TokenStream, item: TokenStream, scope: Scope) -> TokenStream {
    let attr = match StructOrFunctionAttr::from_tokens(attr.into()) {
        Ok(attr) => attr,
        Err(err) => return err.to_compile_error().into(),
    };

    let item = parse_macro_input!(item as Item);

    let result = match item {
        Item::Struct(item_struct) => item_struct_gen::generate(attr, item_struct, scope),
        Item::Enum(item_enum) => item_enum_gen::generate(attr, item_enum, scope),
        Item::Fn(item_fn) => item_fn_gen::generate(attr, item_fn, scope),
        Item::Impl(item_impl) => item_impl_gen::generate(attr, item_impl, scope),
        _ => Err(syn::Error::new(
            item.span(),
            "expected `struct` or `enum` or `function` or `impl block`",
        )),
    };

    result.unwrap_or_else(|e| e.to_compile_error()).into()
}

/// Define a singleton provider.
#[doc = ""]
#[doc = include_str!("./docs/attribute_macro.md")]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Singleton(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate(attr, item, Scope::Singleton)
}

/// Define a transient provider.
#[doc = ""]
#[doc = include_str!("./docs/attribute_macro.md")]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Transient(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate(attr, item, Scope::Transient)
}

/// Define a single owner provider.
#[doc = ""]
#[doc = include_str!("./docs/attribute_macro.md")]
#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn SingleOwner(attr: TokenStream, item: TokenStream) -> TokenStream {
    generate(attr, item, Scope::SingleOwner)
}
