/// Convert a set of types that implement [`Module`]
/// to a set of [`ResolveModule`] instances.
///
/// # Example
///
/// ```rust
/// use rudi::{modules, DynProvider, Module, ResolveModule};
///
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn providers() -> Vec<DynProvider> {
///         Vec::new()
///     }
/// }
///
/// # fn main() {
/// let _: Vec<ResolveModule> = modules![MyModule];
/// # }
/// ```
///
/// [`Module`]: crate::Module
/// [`ResolveModule`]: crate::ResolveModule
#[macro_export]
macro_rules! modules {
    () => {
        vec![]
    };
    ($($module:ty),+ $(,)?) => {
        vec![$(
            $crate::ResolveModule::new::<$module>()
        ),+]
    };
}

/// Convert a set of instances that implement `Into<DynProvider>`
/// to a set of [`DynProvider`] instances
///
/// # Example
///
/// ```rust
/// use rudi::{providers, singleton, DynProvider};
///
/// # fn main() {
/// let _: Vec<DynProvider> = providers![singleton(|_| "Hello")];
/// # }
/// ```
///
/// [`DynProvider`]: crate::DynProvider
#[macro_export]
macro_rules! providers {
    () => {
        vec![]
    };
    ($($provider:expr),+ $(,)?) => {
        vec![$(
            <$crate::DynProvider as ::core::convert::From<_>>::from($provider)
        ),+]
    };
}

/// Convert a set of types that implement [`DefaultProvider`]
/// to a set of [`DynProvider`] instances
///
/// # Example
///
/// ```rust
/// use rudi::{components, DynProvider, Transient};
///
/// #[Transient]
/// struct A;
///
/// # fn main() {
/// let _: Vec<DynProvider> = components![A];
/// # }
/// ```
///
/// [`DefaultProvider`]: crate::DefaultProvider
/// [`DynProvider`]: crate::DynProvider
#[macro_export]
macro_rules! components {
    () => {
        vec![]
    };
    ($($component:ty),+ $(,)?) => {
        vec![$(
            <$crate::DynProvider as ::core::convert::From<_>>::from(
                <$component as $crate::DefaultProvider>::provider()
            )
        ),+]
    };
}
