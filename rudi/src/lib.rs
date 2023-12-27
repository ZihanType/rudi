#![doc = include_str!("./docs/lib.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
#[cfg(feature = "auto-register")]
mod auto_register;
mod context;
mod definition;
mod future;
mod macros;
mod module;
mod provider;
mod registry;
mod single;
mod ty;

#[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
#[cfg(feature = "auto-register")]
pub use auto_register::*;
pub use context::*;
pub use definition::*;
pub use future::*;
pub use module::*;
pub use provider::*;
pub(crate) use registry::*;
pub use rudi_core::*;
pub use single::*;
pub use ty::*;

macro_rules! export_attribute_macros {
    (
        $(
            #[$summary:meta]
            $name:ident;
        )*
    ) => {
        $(
            #[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
            #[cfg(feature = "rudi-macro")]
            #[$summary]
            #[doc = ""]
            #[doc = include_str!("./docs/attribute_macro.md")]
            pub use rudi_macro::$name;
        )*
    };
}

export_attribute_macros! {
    /// Define a singleton provider.
    Singleton;
    /// Define a transient provider.
    Transient;
    /// Define a single owner provider.
    SingleOwner;
}
