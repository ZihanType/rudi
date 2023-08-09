mod item_fn_gen;
mod item_impl_gen;
mod item_struct_gen;
mod name;
mod provider_attribute;
mod utils;

use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, Item};

use crate::{provider_attribute::ProviderAttribute, utils::Scope};

fn macro_attribute(attr: TokenStream, input: TokenStream, scope: Scope) -> TokenStream {
    let attribute = parse_macro_input!(attr as ProviderAttribute);
    let item = parse_macro_input!(input as Item);

    let result = match item {
        Item::Struct(item_struct) => item_struct_gen::generate(attribute, item_struct, scope),
        Item::Fn(item_fn) => item_fn_gen::generate(attribute, item_fn, scope),
        Item::Impl(item_impl) => item_impl_gen::generate(attribute, item_impl, scope),
        _ => Err(syn::Error::new(
            item.span(),
            "expected struct or function or impl",
        )),
    };

    result.unwrap_or_else(|e| e.to_compile_error()).into()
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Singleton(attr: TokenStream, input: TokenStream) -> TokenStream {
    macro_attribute(attr, input, Scope::Singleton)
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Transient(attr: TokenStream, input: TokenStream) -> TokenStream {
    macro_attribute(attr, input, Scope::Transient)
}
