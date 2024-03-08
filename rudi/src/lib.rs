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
#[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
#[cfg(feature = "rudi-macro")]
pub use rudi_macro::*;
pub use single::*;
pub use ty::*;
