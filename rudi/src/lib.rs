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

pub use rudi_core::*;
#[cfg_attr(docsrs, doc(cfg(feature = "rudi-macro")))]
#[cfg(feature = "rudi-macro")]
pub use rudi_macro::*;

#[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
#[cfg(feature = "auto-register")]
pub use self::auto_register::*;
pub(crate) use self::registry::*;
pub use self::{context::*, definition::*, future::*, module::*, provider::*, single::*, ty::*};
