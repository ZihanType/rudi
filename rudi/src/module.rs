use std::any::{self, TypeId};

use crate::provider::DynProvider;

/// Represents a module.
///
/// # Example
///
/// ```rust
/// use rudi::{modules, providers, singleton, transient, Context, Module};
///
/// struct Module1;
///
/// impl Module for Module1 {
///     fn eager_create() -> bool {
///         true
///     }
///
///     fn providers() -> Vec<rudi::DynProvider> {
///         providers![singleton(|_| "Hello").name("1")]
///     }
/// }
///
/// struct Module2;
///
/// impl Module for Module2 {
///     fn submodules() -> Option<Vec<rudi::ResolveModule>> {
///         Some(modules![Module1])
///     }
///
///     fn providers() -> Vec<rudi::DynProvider> {
///         providers![transient(|_| "World").name("2")]
///     }
/// }
///
/// # fn main() {
/// let mut cx = Context::create(modules![Module2]);
/// let mut a = cx.resolve_by_type::<&'static str>();
/// a.sort();
/// assert!(format!("{:?}", a) == *r#"["Hello", "World"]"#);
/// # }
/// ```
pub trait Module {
    /// Whether the providers included in the module should be created eagerly, default is false.
    fn eager_create() -> bool {
        false
    }

    /// Included submodules, default is None.
    fn submodules() -> Option<Vec<ResolveModule>> {
        None
    }

    /// Included providers.
    fn providers() -> Vec<DynProvider>;
}

/// A type representing a Module, converted from a type that implements [`Module`].
pub struct ResolveModule {
    id: TypeId,
    name: &'static str,
    eager_create: bool,
    submodules: Option<Vec<ResolveModule>>,
    providers: Vec<DynProvider>,
}

impl ResolveModule {
    /// Create a [`ResolveModule`] from a type that implements [`Module`].
    pub fn new<T: Module + 'static>() -> Self {
        Self {
            id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
            eager_create: T::eager_create(),
            submodules: T::submodules(),
            providers: T::providers(),
        }
    }

    /// TypeId of the type that is converted to a ResolveModule.
    pub fn id(&self) -> TypeId {
        self.id
    }

    /// Type name of the type that is converted to a ResolveModule.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Whether the providers included in the module should be created eagerly.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    pub(crate) fn submodules(&mut self) -> Option<Vec<ResolveModule>> {
        self.submodules.take()
    }

    pub(crate) fn providers(self) -> Vec<DynProvider> {
        self.providers
    }
}
