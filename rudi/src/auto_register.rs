#[doc(hidden)]
pub use inventory::submit;

use crate::{module::Module, provider::DynProvider};

#[doc(hidden)]
pub struct ProviderRegister {
    pub register: fn() -> DynProvider,
}

inventory::collect!(ProviderRegister);

/// Returns an iterator over all auto-registered providers.
///
/// [`AutoRegisterModule`] uses this function to collect all auto-registered [`DynProvider`]s.
/// If you don't want to use `AutoRegisterModule`, you can use this function to customize your own module.
///
/// # Example
///
/// ```rust
/// use rudi::{auto_registered_providers, Module};
///
/// struct MyAutoRegisterModule;
///
/// impl Module for MyAutoRegisterModule {
///     fn eager_create() -> bool {
///         true
///     }
///
///     fn providers() -> Vec<rudi::DynProvider> {
///         auto_registered_providers().collect()
///     }
/// }
/// ```
pub fn auto_registered_providers() -> impl Iterator<Item = DynProvider> {
    inventory::iter::<ProviderRegister>
        .into_iter()
        .map(|register| (register.register)())
}

/// A module that auto-registers all providers.
///
/// This module is enabled by the `auto-register` feature.
/// Because auto-registration relies on [`inventory`] crate, auto-registration
/// is not available on platforms where `inventory` is not supported.
///
/// # Example
///
/// ```rust
/// use rudi::{Context, Singleton, Transient};
///
/// #[Singleton]
/// #[derive(Clone)]
/// struct A;
///
/// #[Transient]
/// struct B(A);
///
/// # fn main() {
/// let mut cx = Context::auto_register();
/// assert!(cx.resolve_option::<B>().is_some());
/// # }
/// ```
pub struct AutoRegisterModule;

impl Module for AutoRegisterModule {
    fn providers() -> Vec<DynProvider> {
        auto_registered_providers().collect()
    }
}

/// Register a `Provider` that will be collected by [`auto_registered_providers`].
///
/// If you have:
///   - Enabled the `auto-register` feature (which is enabled by default).
///   - Define `Provider` using the `#[Singleton]` or `#[Transient]` macro.
///   - `#[Singleton]` or `#[Transient]` does not use the `not_auto_register` attribute.
///
/// Then you don't need to use this macro to register `Provider`.
///
/// But if you use function define a `Provider` and you want to use auto-registration,
/// then you need to use this macro.
///
/// # Example
///
/// ```rust
/// use rudi::{register_provider, singleton, Context, Provider};
///
/// fn foo() -> Provider<&'static str> {
///     singleton(|_| "Hello").into()
/// }
///
/// register_provider!(foo());
///
/// fn main() {
///     let mut cx = Context::auto_register();
///     assert!(cx.resolve_option::<&'static str>().is_some());
/// }
/// ```
#[macro_export]
macro_rules! register_provider {
    ($provider:expr) => {
        const _: () = {
            fn register() -> $crate::DynProvider {
                <$crate::DynProvider as ::core::convert::From<_>>::from($provider)
            }

            $crate::submit! {
                $crate::ProviderRegister {
                    register
                }
            }
        };
    };
}

/// Generate a function to enable auto-registration.
///
/// In Rust, it is possible to use [`inventory`] to accomplish something like
/// auto-registration, but there is still a problem, and it exists in Rudi as well.
///
/// Suppose you have two crates, one called `crate_1` and one called `crate_2`,
/// and you define some auto-registration types in `crate_2`.
///
/// If it is just a dependency on `crate_2` in `crate_1`'s `Cargo.toml`, then using
/// [`auto_registered_providers`] in `crate_1` will not collect the types defined in `crate_2`,
/// you have to use a function (or type, or constant) in `crate_1` that is defined in `crate_2`
/// in order to enable auto-registration.
///
/// So, there is this macro, which generates a function called `enable`, with no parameters
/// and no return, just to be called by other crates to enable auto-registration.
///
/// At the same time, you can also call the enable functions of other crates that the current
/// crate depends on in this macro, so that when the enable function of the current crate is
/// called, the enable functions of other crates will be called together.
///
/// # Example
///
/// ```rust ignore
/// // lib1/src/lib.rs
/// use rudi::{enable, Transient};
///
/// enable! {}
///
/// #[Transient(name = "lib1")]
/// fn lib1() -> i32 {
///     5
/// }
///
/// // lib2/src/lib.rs
/// use rudi::{enable, Transient};
///
/// enable! {
///     lib1::enable();
/// }
///
/// #[Transient(name = "lib2")]
/// fn lib2() -> i32 {
///     5
/// }
///
/// // bin/src/main.rs
/// use rudi::*;
///
/// fn main() {
///     lib2::enable();
///
///     let mut cx = Context::auto_register();
///     assert_eq!(cx.resolve_by_type::<i32>().into_iter().sum::<i32>(), 10);
/// }
/// ```
#[macro_export]
macro_rules! enable {
    ($($body:tt)*) => {
        /// Enable auto-registration.
        pub fn enable() {
            $($body)*
        }
    };
}
