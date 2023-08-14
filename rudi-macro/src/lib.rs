mod commons;
mod field_or_argument_attributes;
mod item_fn_gen;
mod item_impl_gen;
mod item_struct_gen;
mod struct_or_function_attributes;

use proc_macro::TokenStream;
use syn::{parse_macro_input, spanned::Spanned, Item};

use crate::{commons::Scope, struct_or_function_attributes::StructOrFunctionAttributes};

fn macro_attribute(args: TokenStream, input: TokenStream, scope: Scope) -> TokenStream {
    let mut attrs = StructOrFunctionAttributes::default();
    let parser = syn::meta::parser(|meta| attrs.parse(meta));
    parse_macro_input!(args with parser);
    let item = parse_macro_input!(input as Item);

    let result = match item {
        Item::Struct(item_struct) => item_struct_gen::generate(attrs, item_struct, scope),
        Item::Fn(item_fn) => item_fn_gen::generate(attrs, item_fn, scope),
        Item::Impl(item_impl) => item_impl_gen::generate(attrs, item_impl, scope),
        _ => Err(syn::Error::new(
            item.span(),
            "expected struct or function or impl",
        )),
    };

    result.unwrap_or_else(|e| e.to_compile_error()).into()
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Singleton(args: TokenStream, input: TokenStream) -> TokenStream {
    macro_attribute(args, input, Scope::Singleton)
}

#[proc_macro_attribute]
#[allow(non_snake_case)]
pub fn Transient(args: TokenStream, input: TokenStream) -> TokenStream {
    macro_attribute(args, input, Scope::Transient)
}
