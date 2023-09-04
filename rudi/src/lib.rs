#![doc = include_str!("./docs/lib.md")]
#![forbid(unsafe_code)]
#![deny(private_interfaces, private_bounds, unreachable_pub)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(missing_docs)]

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
mod singleton;
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
#[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
#[cfg(feature = "rudi-macro")]
/// Define a singleton provider.
#[doc = include_str!("docs/attribute_macro.md")]
pub use rudi_macro::Singleton;
#[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
#[cfg(feature = "rudi-macro")]
/// Define a transient provider.
#[doc = include_str!("docs/attribute_macro.md")]
pub use rudi_macro::Transient;
pub use singleton::*;
pub use ty::*;
