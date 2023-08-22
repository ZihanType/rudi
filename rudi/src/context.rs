use std::{any::TypeId, borrow::Cow, collections::hash_map::Iter, rc::Rc};

use crate::{
    module::ResolveModule,
    provider::{DynProvider, Provider},
    BoxFuture, Constructor, Definition, EagerCreateFunction, Key, ProviderRegistry, Scope,
    SingletonInstance, SingletonRegistry,
};

/// A context is a container for all the providers and singletons.
///
/// It is the main entry point for the dependency injection.
/// It is also used to create new instances.
///
/// When creating a `Context`, you can use options to change the
/// default creation behavior, see [`ContextOptions`] for details.
///
/// # Example
///
/// Creating context with customized modules:
/// ```rust
/// use rudi::{components, modules, Context, Module, Transient};
///
/// #[Transient]
/// struct A;
///
/// struct Module1;
///
/// impl Module for Module1 {
///     fn providers() -> Vec<rudi::DynProvider> {
///         components![A]
///     }
/// }
///
/// #[derive(Debug)]
/// #[Transient]
/// struct B;
///
/// struct Module2;
///
/// impl Module for Module2 {
///     fn providers() -> Vec<rudi::DynProvider> {
///         components![B]
///     }
/// }
///
/// # fn main() {
/// let mut cx = Context::create(modules![Module1, Module2]);
///
/// let b = cx.resolve::<B>();
///
/// assert!(cx.resolve_option::<A>().is_some());
/// assert_eq!(format!("{:?}", b), "B");
/// # }
/// ```
///
/// With the `auto-register` feature enabled (which is enabled by default),
/// it is also possible to create contexts in a simpler way:
/// ```rust
/// use rudi::{Context, Transient};
///
/// #[Transient]
/// struct A;
///
/// #[derive(Debug)]
/// #[Transient]
/// struct B;
///
/// # fn main() {
/// let mut cx = Context::auto_register();
/// // This is a simplified version of the following
/// // let mut cx = Context::create(modules![AutoRegisterModule]);
///
/// let b = cx.resolve::<B>();
///
/// assert!(cx.resolve_option::<A>().is_some());
/// assert_eq!(format!("{:?}", b), "B");
/// # }
/// ```
///
/// If the following conditions are met:
/// 1. in context, there exists a provider whose constructor is async.
/// 2. the `eager_create` method of the provider is set to true, e.g., [`SingletonProvider::eager_create`](crate::SingletonProvider::eager_create).
/// 3. the `eager_create` method of the module to which the provide belongs is set to true, i.e., [`Module::eager_create`](crate::Module::eager_create).
/// 4. the `eager_create` field of the context, is set to true, i.e., [`ContextOptions::eager_create`].
///
/// Then when creating the context, you must use the async creation methods, [`Context::create_async`] or [`Context::auto_register_async`]:
///
/// ```rust
/// use rudi::{Context, Singleton, Transient};
///
/// #[Singleton]
/// async fn Foo() -> i32 {
///     1
/// }
///
/// #[derive(Debug)]
/// #[Transient(async)]
/// struct A(i32);
///
/// #[tokio::main]
/// async fn main() {
///     let mut cx = Context::options()
///         .eager_create(true)
///         .auto_register_async()
///         .await;
///
///     assert!(cx.resolve_option_async::<A>().await.is_some());
/// }
/// ```
pub struct Context {
    allow_override: bool,
    allow_only_singleton_eager_create: bool,

    eager_create: bool,

    singleton_registry: SingletonRegistry,
    provider_registry: ProviderRegistry,

    eager_create_functions: Vec<(Definition, EagerCreateFunction)>,
    dependency_chain: DependencyChain,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_singleton_eager_create: true,
            eager_create: Default::default(),
            singleton_registry: Default::default(),
            provider_registry: Default::default(),
            eager_create_functions: Default::default(),
            dependency_chain: Default::default(),
        }
    }
}

impl Context {
    /// Creates a new context with the given modules.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider whose constructor is async and the context's `eager_create`
    /// or the module's `eager_create` or the provider's `eager_create` is set to true.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{components, modules, Context, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<rudi::DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let mut cx = Context::create(modules![MyModule]);
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn create(modules: Vec<ResolveModule>) -> Context {
        ContextOptions::default().create(modules)
    }

    /// Creates a new context with the [`AutoRegisterModule`].
    ///
    /// Same as `Context::create(modules![AutoRegisterModule])`.
    ///
    /// See [`Context::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider whose constructor is async and the context's `eager_create`
    /// or the module's `eager_create` or the provider's `eager_create` is set to true.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// [`AutoRegisterModule`]: crate::AutoRegisterModule
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[cfg(feature = "auto-register")]
    #[track_caller]
    pub fn auto_register() -> Context {
        ContextOptions::default().auto_register()
    }

    /// Async version of [`Context::create`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`Context::create`].
    ///
    /// See [`Context::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_async(modules: Vec<ResolveModule>) -> Context {
        ContextOptions::default().create_async(modules).await
    }

    /// Async version of [`Context::auto_register`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`Context::auto_register`].
    ///
    /// See [`Context::auto_register`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider that panics on construction.
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[cfg(feature = "auto-register")]
    pub async fn auto_register_async() -> Context {
        ContextOptions::default().auto_register_async().await
    }

    /// Returns a new ContextOptions object.
    ///
    /// This function return a new ContextOptions object that you can use to create a context with specific options
    /// if `create()` or `auto_register()` are not appropriate.
    ///
    /// It is equivalent to `ContextOptions::default()`, but allows you to write more readable code.
    /// Instead of `ContextOptions::default().eager_create(true).auto_register()`,
    /// you can write `Context::options().eager_create(true).auto_register()`.
    /// This also avoids the need to import `ContextOptions`.
    ///
    /// See the [`ContextOptions`] for more details.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::options().eager_create(true).auto_register();
    ///
    /// assert!(cx.contains_singleton::<A>());
    /// # }
    /// ```
    pub fn options() -> ContextOptions {
        ContextOptions::default()
    }

    /// Load the given modules.
    ///
    /// This method first flattens all the given modules together with their submodules
    /// into a collection of modules without submodules, then takes out the providers of
    /// each module in this collection, flattens all the providers together with their
    /// bound providers into a collection of providers without bound providers, and finally
    /// deposits the providers one by one into context.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    /// assert!(cx.get_provider::<A>().is_none());
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn load_modules(&mut self, modules: Vec<ResolveModule>) {
        let Some(modules) = flatten(modules, ResolveModule::submodules) else {
            return;
        };

        modules.into_iter().for_each(|module| {
            self.load_providers(module.eager_create(), module.providers());
        });
    }

    /// Unload the given modules.
    ///
    /// This method will convert the given module into a collection of providers like
    /// the [`Context::load_modules`] method, and then remove all providers in the context
    /// that are equal to the providers in the collection and their possible singletons.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    /// assert!(cx.get_provider::<A>().is_none());
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_some());
    ///
    /// cx.unload_modules(modules![AutoRegisterModule]);
    /// assert!(cx.get_provider::<A>().is_none());
    /// # }
    /// ```
    pub fn unload_modules(&mut self, modules: Vec<ResolveModule>) {
        let Some(modules) = flatten(modules, ResolveModule::submodules) else {
            return;
        };

        modules.into_iter().for_each(|module| {
            self.unload_providers(module.providers());
        });
    }

    /// Create instances where `eager_create` is true.
    ///
    /// When the provider is loaded into the context, an or operation will be performed
    /// on the `eager_create` value of the provider, the module to which the provider belongs,
    /// and the context to arrive at the final `eager_create` value, and if it is true,
    /// then the constructor of the provider will be pushed to a queue. When this method is called,
    /// the constructor is taken from this queue and executed.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async and the context's `eager_create`
    /// or the module's `eager_create` or the provider's `eager_create` is set to true.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    /// assert!(!cx.contains_singleton::<A>());
    ///
    /// cx.create_eager_instances();
    /// assert!(cx.contains_singleton::<A>());
    /// # }
    /// ```
    #[track_caller]
    pub fn create_eager_instances(&mut self) {
        if self.eager_create_functions.is_empty() {
            return;
        }

        self.eager_create_functions.reverse();

        while let Some((definition, eager_create_function)) = self.eager_create_functions.pop() {
            match eager_create_function {
                EagerCreateFunction::Async(_) => {
                    panic!(
                        "unable to call an async eager create function in a sync context for: {:?}

please use instead:
1. Context::create_async(modules).await
2. Context::auto_register_async().await
3. ContextOptions::create_async(options, modules).await
4. ContextOptions::auto_register_async(options).await
",
                        definition
                    )
                }
                EagerCreateFunction::Sync(eager_create_function) => {
                    eager_create_function(self, definition.key.name)
                }
            }
        }
    }

    /// Async version of [`Context::create_eager_instances`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`Context::create_eager_instances`].
    ///
    /// See [`Context::create_eager_instances`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_eager_instances_async(&mut self) {
        if self.eager_create_functions.is_empty() {
            return;
        }

        self.eager_create_functions.reverse();

        while let Some((definition, eager_create_function)) = self.eager_create_functions.pop() {
            match eager_create_function {
                EagerCreateFunction::Async(eager_create_function) => {
                    eager_create_function(self, definition.key.name).await
                }
                EagerCreateFunction::Sync(eager_create_function) => {
                    eager_create_function(self, definition.key.name)
                }
            }
        }
    }

    /// Returns an instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    /// let a = cx.resolve::<A>();
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve<T: 'static>(&mut self) -> T {
        self.resolve_with_name("")
    }

    /// Returns an instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    /// let a = cx.resolve_with_name::<A>("a");
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_with_name<T: 'static>(&mut self, name: impl Into<Cow<'static, str>>) -> T {
        let key = Key::new::<T>(name.into());
        self.resolve_keyed(key.clone())
            .unwrap_or_else(|| panic!("no provider registered for: {:?}", key))
    }

    /// Returns an optional instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_option<T: 'static>(&mut self) -> Option<T> {
        self.resolve_option_with_name("")
    }

    /// Returns an optional instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    /// assert!(cx.resolve_option_with_name::<A>("a").is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_option_with_name<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<T> {
        let key = Key::new::<T>(name.into());
        self.resolve_keyed(key)
    }

    /// Returns a collection of instances of the given type.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    /// assert_eq!(cx.resolve_by_type::<i32>().into_iter().sum::<i32>(), 3);
    /// # }
    /// ```
    #[track_caller]
    pub fn resolve_by_type<T: 'static>(&mut self) -> Vec<T> {
        self.keys::<T>()
            .into_iter()
            .filter_map(|key| self.resolve_keyed(key))
            .collect()
    }

    /// Async version of [`Context::resolve`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async)]
    /// struct A(i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert_eq!(cx.resolve_async::<i32>().await, 1);
    ///     assert!(cx.resolve_option_async::<A>().await.is_some());
    /// }
    /// ```
    pub async fn resolve_async<T: 'static>(&mut self) -> T {
        self.resolve_with_name_async("").await
    }

    /// Async version of [`Context::resolve_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async, name = "A")]
    /// struct A(#[di(name = "a")] i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert_eq!(cx.resolve_with_name_async::<i32>("a").await, 1);
    ///     assert!(cx.resolve_option_with_name_async::<A>("A").await.is_some());
    /// }
    /// ```
    pub async fn resolve_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> T {
        let key = Key::new::<T>(name.into());
        self.resolve_keyed_async(key.clone())
            .await
            .unwrap_or_else(|| panic!("no provider registered for: {:?}", key))
    }

    /// Async version of [`Context::resolve_option`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async)]
    /// struct A(i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert_eq!(cx.resolve_async::<i32>().await, 1);
    ///     assert!(cx.resolve_option_async::<A>().await.is_some());
    /// }
    /// ```
    pub async fn resolve_option_async<T: 'static>(&mut self) -> Option<T> {
        self.resolve_option_with_name_async("").await
    }

    /// Async version of [`Context::resolve_option_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn Number() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(async, name = "A")]
    /// struct A(#[di(name = "a")] i32);
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert_eq!(cx.resolve_with_name_async::<i32>("a").await, 1);
    ///     assert!(cx.resolve_option_with_name_async::<A>("A").await.is_some());
    /// }
    /// ```
    pub async fn resolve_option_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<T> {
        let key = Key::new::<T>(name.into());
        self.resolve_keyed_async(key).await
    }

    /// Async version of [`Context::resolve_by_type`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// async fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// async fn B() -> i32 {
    ///     2
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert_eq!(
    ///         cx.resolve_by_type_async::<i32>()
    ///             .await
    ///             .into_iter()
    ///             .sum::<i32>(),
    ///         3
    ///     );
    /// }
    /// ```
    pub async fn resolve_by_type_async<T: 'static>(&mut self) -> Vec<T> {
        let keys = self.keys::<T>();

        let mut instances = Vec::with_capacity(keys.len());

        for key in keys {
            if let Some(instance) = self.resolve_keyed_async(key).await {
                instances.push(instance);
            }
        }

        instances
    }

    /// Returns true if the context contains a provider for the specified type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.contains_provider::<A>());
    /// # }
    /// ```
    pub fn contains_provider<T: 'static>(&self) -> bool {
        self.contains_provider_with_name::<T>("")
    }

    /// Returns true if the context contains a provider for the specified type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.contains_provider_with_name::<A>("a"));
    /// # }
    /// ```
    pub fn contains_provider_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        let key = Key::new::<T>(name.into());
        self.provider_registry.contains(&key)
    }

    /// Returns a reference to an provider based on the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.get_provider::<A>().is_some());
    /// # }
    /// ```
    pub fn get_provider<T: 'static>(&self) -> Option<&Provider<T>> {
        self.get_provider_with_name("")
    }

    /// Returns a reference to an provider based on the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.get_provider_with_name::<A>("a").is_some());
    /// # }
    /// ```
    pub fn get_provider_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&Provider<T>> {
        let key = Key::new::<T>(name.into());
        self.provider_registry.get(&key)
    }

    /// Returns a collection of references to providers based on the given type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient(name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let cx = Context::auto_register();
    ///     assert_eq!(cx.get_providers_by_type::<i32>().len(), 2);
    /// }
    /// ```
    pub fn get_providers_by_type<T: 'static>(&self) -> Vec<&Provider<T>> {
        let type_id = TypeId::of::<T>();

        self.iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, provider)| provider.as_provider())
            .collect()
    }

    /// An iterator visiting all Key-DynProvider pairs in arbitrary order. The iterator element type is (&'a Key, &'a DynProvider).
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    ///
    /// for (key, _provider) in cx.iter() {
    ///     println!("{:?}", key);
    /// }
    /// # }
    /// ```
    pub fn iter(&self) -> Iter<'_, Key, DynProvider> {
        self.provider_registry.iter()
    }

    /// Returns the number of providers in the context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert_eq!(cx.providers_len(), 2);
    /// # }
    /// ```
    pub fn providers_len(&self) -> usize {
        self.provider_registry.len()
    }

    /// Returns true if the context contains a singleton for the specified type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.contains_singleton::<A>());
    /// # }
    /// ```
    pub fn contains_singleton<T: 'static>(&self) -> bool {
        self.contains_singleton_with_name::<T>("")
    }

    /// Returns true if the context contains a singleton for the specified type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.contains_singleton_with_name::<A>("a"));
    /// # }
    /// ```
    pub fn contains_singleton_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        let key = Key::new::<T>(name.into());
        self.singleton_registry.contains(&key)
    }

    /// Returns the number of singletons in the context.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert_eq!(cx.singletons_len(), 1);
    /// # }
    /// ```
    pub fn singletons_len(&self) -> usize {
        self.singleton_registry.len()
    }
}

impl Context {
    #[track_caller]
    fn load_providers(&mut self, eager_create: bool, providers: Vec<DynProvider>) {
        let Some(providers) = flatten(providers, DynProvider::binding_providers) else {
            return;
        };

        providers.into_iter().for_each(|provider| {
            if !(provider.condition())(self) {
                #[cfg(feature = "debug-print")]
                tracing::warn!("(×) condition not met: {:?}", provider.definition());
                return;
            }

            let need_eager_create = self.eager_create || eager_create || provider.eager_create();

            let allow_all_scope = !self.allow_only_singleton_eager_create;
            let allow_only_singleton_and_it_is_singleton = self.allow_only_singleton_eager_create
                && matches!(provider.definition().scope, Scope::Singleton);

            let allow_eager_create = allow_all_scope || allow_only_singleton_and_it_is_singleton;

            if need_eager_create && allow_eager_create {
                self.eager_create_functions.push((
                    provider.definition().clone(),
                    provider.eager_create_function(),
                ));
            }

            self.provider_registry.insert(provider, self.allow_override);
        });
    }

    fn unload_providers(&mut self, providers: Vec<DynProvider>) {
        let Some(providers) = flatten(providers, DynProvider::binding_providers) else {
            return;
        };

        providers.into_iter().for_each(|provider| {
            let key = provider.key();
            self.provider_registry.remove(key);
            self.singleton_registry.remove(key);
        });
    }

    #[track_caller]
    fn resolve_keyed<T: 'static>(&mut self, key: Key) -> Option<T> {
        let singleton = self.singleton_registry.get::<T>(&key);
        if singleton.is_some() {
            return singleton;
        }

        let provider = self.provider_registry.get(&key)?;
        let constructor = provider.constructor();
        let clone_instance = provider.clone_instance();

        let instance = match constructor {
            Constructor::Async(_) => {
                panic!(
                    "unable to call an async constructor in a sync context for: {:?}

please check all the references to the above type, there are 3 scenarios that will be referenced:
1. use `Context::resolve_xxx::<Type>(cx)` to get instances of the type, change to `Context::resolve_xxx_async::<Type>(cx).await`.
2. use `yyy: Type` as a field of a struct, or a field of a variant of a enum, use `#[Singleton(async)]` or `#[Transient(async)]` on the struct or enum.
3. use `zzz: Type` as a argument of a function, add the `async` keyword to the function.
",
                    provider.definition()
                )
            }
            Constructor::Sync(constructor) => self.resolve_instance(key.clone(), constructor),
        };

        if let Some(clone_instance) = clone_instance {
            self.singleton_registry
                .insert(key, SingletonInstance::new(&instance, clone_instance));
        }

        Some(instance)
    }

    async fn resolve_keyed_async<T: 'static>(&mut self, key: Key) -> Option<T> {
        let singleton = self.singleton_registry.get::<T>(&key);
        if singleton.is_some() {
            return singleton;
        }

        let provider = self.provider_registry.get(&key)?;
        let constructor = provider.constructor();
        let clone_instance = provider.clone_instance();

        let instance = match constructor {
            Constructor::Async(constructor) => {
                self.resolve_instance_async(key.clone(), constructor).await
            }
            Constructor::Sync(constructor) => self.resolve_instance(key.clone(), constructor),
        };

        if let Some(clone_instance) = clone_instance {
            self.singleton_registry
                .insert(key, SingletonInstance::new(&instance, clone_instance));
        }

        Some(instance)
    }

    #[track_caller]
    fn resolve_instance<T: 'static>(
        &mut self,
        key: Key,
        constructor: Rc<dyn Fn(&mut Context) -> T>,
    ) -> T {
        self.dependency_chain.push(key);
        let instance = constructor(self);
        self.dependency_chain.pop();
        instance
    }

    #[allow(clippy::type_complexity)]
    async fn resolve_instance_async<T: 'static>(
        &mut self,
        key: Key,
        constructor: Rc<dyn for<'a> Fn(&'a mut Context) -> BoxFuture<'a, T>>,
    ) -> T {
        self.dependency_chain.push(key);
        let instance = constructor(self).await;
        self.dependency_chain.pop();
        instance
    }

    fn keys<T: 'static>(&self) -> Vec<Key> {
        let type_id = TypeId::of::<T>();

        self.provider_registry
            .keys()
            .filter(|&key| key.ty.id == type_id)
            .cloned()
            .collect()
    }
}

fn flatten<T, F>(mut unresolved: Vec<T>, get_sublist: F) -> Option<Vec<T>>
where
    F: Fn(&mut T) -> Option<Vec<T>>,
{
    if unresolved.is_empty() {
        return None;
    }

    let mut resolved = Vec::with_capacity(unresolved.len());

    unresolved.reverse();

    while let Some(mut element) = unresolved.pop() {
        match get_sublist(&mut element) {
            Some(mut sublist) if !sublist.is_empty() => {
                sublist.reverse();
                unresolved.append(&mut sublist);
            }
            _ => {}
        }

        resolved.push(element);
    }

    Some(resolved)
}

/// Options and flags which can be used to configure how a context is created.
///
/// This builder expose the ability to configure how a [`Context`] is created.
/// The [`Context::create`] and [`Context::auto_register`] methods are aliases
/// for commonly used options using this builder.
///
/// Generally speaking, when using `ContextOptions`, you'll first call [`ContextOptions::default`],
/// then chain calls to methods to set each option, then call [`ContextOptions::create`], paasing the modules you've built,
/// or call [`ContextOptions::auto_register`]. This will give you a [`Context`].
///
/// # Example
///
/// Creating a context with a module:
///
/// ```rust
/// use rudi::{modules, Context, ContextOptions, Module};
///
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn providers() -> Vec<rudi::DynProvider> {
///         vec![]
///     }
/// }
///
/// # fn main() {
/// let _cx: Context = ContextOptions::default().create(modules![MyModule]);
/// # }
/// ```
///
/// Creating a context with [`AutoRegisterModule`]:
///
/// ```rust
/// use rudi::{modules, AutoRegisterModule, Context, ContextOptions};
///
/// # fn main() {
/// let _cx: Context = ContextOptions::default().create(modules![AutoRegisterModule]);
/// // or use simpler method
/// // let _cx: Context = ContextOptions::default().auto_register();
/// # }
/// ```
///
/// Creating a context with both options:
///
/// ```rust
/// use rudi::{modules, AutoRegisterModule, Context, ContextOptions};
///
/// # fn main() {
/// let _cx: Context = ContextOptions::default()
///     .allow_override(true)
///     .allow_only_singleton_eager_create(true)
///     .eager_create(false)
///     .instance(42)
///     .instance_with_name("Hello", "str_instance_1")
///     .instance_with_name("World", "str_instance_2")
///     .create(modules![AutoRegisterModule]);
/// # }
/// ```
///
/// [`AutoRegisterModule`]: crate::AutoRegisterModule
pub struct ContextOptions {
    allow_override: bool,
    allow_only_singleton_eager_create: bool,
    eager_create: bool,
    providers: Vec<DynProvider>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_singleton_eager_create: true,
            eager_create: Default::default(),
            providers: Default::default(),
        }
    }
}

impl ContextOptions {
    /// Sets the option for whether the context should allow overriding existing providers.
    ///
    /// This option, when true, allows a provider to override an existing provider with the same key.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let _cx: Context = ContextOptions::default()
    ///     .allow_override(true)
    ///     .auto_register();
    /// # }
    /// ```
    pub fn allow_override(mut self, allow_override: bool) -> Self {
        self.allow_override = allow_override;
        self
    }

    /// Sets the option for whether the context should only eagerly create singleton instances.
    ///
    /// This option, when true, will only eagerly create instances for singleton providers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let _cx: Context = ContextOptions::default()
    ///     .allow_only_singleton_eager_create(true)
    ///     .auto_register();
    /// # }
    /// ```
    pub fn allow_only_singleton_eager_create(
        mut self,
        allow_only_singleton_eager_create: bool,
    ) -> Self {
        self.allow_only_singleton_eager_create = allow_only_singleton_eager_create;
        self
    }

    /// Sets the option for whether the context should eagerly create instances.
    ///
    /// This option, when true, will eagerly create instances for all providers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let _cx: Context = ContextOptions::default()
    ///     .eager_create(false)
    ///     .auto_register();
    /// # }
    /// ```
    pub fn eager_create(mut self, eager_create: bool) -> Self {
        self.eager_create = eager_create;
        self
    }

    /// Appends a standalone instance to the context with default name `""`.
    ///
    /// This method is used to add certain constants to the context.
    ///
    /// # Panics
    ///
    /// - Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let mut cx: Context = ContextOptions::default().instance(42).auto_register();
    /// assert_eq!(cx.resolve::<i32>(), 42);
    /// # }
    /// ```
    pub fn instance<T>(self, instance: T) -> Self
    where
        T: 'static + Clone,
    {
        self.instance_with_name(instance, "")
    }

    /// Appends a standalone instance to the context with name.
    ///
    /// This method is used to add certain constants to the context.
    ///
    /// # Panics
    ///
    /// - Panics if the new capacity exceeds `isize::MAX` bytes.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let mut cx: Context = ContextOptions::default()
    ///     .instance_with_name(1, "i32_instance_1")
    ///     .instance_with_name(2, "i32_instance_2")
    ///     .auto_register();
    /// assert_eq!(cx.resolve_by_type::<i32>().into_iter().sum::<i32>(), 3);
    /// # }
    /// ```
    pub fn instance_with_name<T, N>(mut self, instance: T, name: N) -> Self
    where
        T: 'static + Clone,
        N: Into<Cow<'static, str>>,
    {
        let provider = Provider::standalone(name.into(), instance).into();
        self.providers.push(provider);
        self
    }

    fn inner_create<F>(self, init: F) -> Context
    where
        F: FnOnce(&mut Context),
    {
        let ContextOptions {
            allow_override,
            allow_only_singleton_eager_create,
            eager_create,
            providers,
        } = self;

        let mut cx = Context {
            allow_override,
            allow_only_singleton_eager_create,
            eager_create,
            ..Default::default()
        };

        if !providers.is_empty() {
            cx.load_providers(false, providers);
        }

        init(&mut cx);

        cx
    }

    /// Creates a new context with the given modules.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider whose constructor is async and the context's `eager_create`
    /// or the module's `eager_create` or the provider's `eager_create` is set to true.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{components, modules, Context, ContextOptions, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<rudi::DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// # fn main() {
    /// let mut cx: Context = ContextOptions::default().create(modules![MyModule]);
    /// assert!(cx.resolve_option::<A>().is_some());
    /// # }
    /// ```
    #[track_caller]
    pub fn create(self, modules: Vec<ResolveModule>) -> Context {
        let mut cx = self.inner_create(|cx| cx.load_modules(modules));
        cx.create_eager_instances();
        cx
    }

    /// Creates a new context with the [`AutoRegisterModule`].
    ///
    /// Same as `ContextOptions::default().create(modules![AutoRegisterModule])`.
    ///
    /// See [`ContextOptions::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider whose constructor is async and the context's `eager_create`
    /// or the module's `eager_create` or the provider's `eager_create` is set to true.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// [`AutoRegisterModule`]: crate::AutoRegisterModule
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[cfg(feature = "auto-register")]
    #[track_caller]
    pub fn auto_register(self) -> Context {
        use crate::AutoRegisterModule;

        let mut cx = self.inner_create(|cx| {
            let module = ResolveModule::new::<AutoRegisterModule>();
            cx.load_providers(module.eager_create(), module.providers())
        });

        cx.create_eager_instances();
        cx
    }

    /// Async version of [`ContextOptions::create`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ContextOptions::create`].
    ///
    /// See [`ContextOptions::create`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_async(self, modules: Vec<ResolveModule>) -> Context {
        let mut cx = self.inner_create(|cx| cx.load_modules(modules));
        cx.create_eager_instances_async().await;
        cx
    }

    /// Async version of [`ContextOptions::auto_register`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`ContextOptions::auto_register`].
    ///
    /// See [`ContextOptions::auto_register`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's `allow_override` is set to false.
    /// - Panics if there is a provider that panics on construction.
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[cfg(feature = "auto-register")]
    pub async fn auto_register_async(self) -> Context {
        use crate::AutoRegisterModule;

        let mut cx = self.inner_create(|cx| {
            let module = ResolveModule::new::<AutoRegisterModule>();
            cx.load_providers(module.eager_create(), module.providers())
        });

        cx.create_eager_instances_async().await;
        cx
    }
}

#[derive(Default)]
struct DependencyChain {
    stack: Vec<Key>,
}

impl DependencyChain {
    fn push(&mut self, key: Key) {
        let already_contains = self.stack.contains(&key);
        self.stack.push(key);

        if already_contains {
            let key = self.stack.last().unwrap();

            let mut buf = String::with_capacity(1024);
            buf.push('[');
            buf.push('\n');

            self.stack.iter().for_each(|k| {
                if key == k {
                    buf.push_str(" --> ")
                } else {
                    buf.push_str("     ")
                }

                buf.push_str(format!("{:?}", k).as_str());
                buf.push('\n');
            });

            buf.push(']');

            panic!("circular dependency detected: {}", buf);
        }
    }

    fn pop(&mut self) {
        self.stack.pop();
    }
}
