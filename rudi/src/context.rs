use std::{any::TypeId, borrow::Cow, collections::HashMap, rc::Rc};

use crate::{
    BoxFuture, Constructor, Definition, DynProvider, DynSingletonInstance, EagerCreateFunction,
    Key, Provider, ProviderRegistry, ResolveModule, Scope, SingletonInstance, SingletonRegistry,
    Type,
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

    loaded_modules: Vec<Type>,
    conditional_providers: Vec<(bool, DynProvider)>,
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
            loaded_modules: Default::default(),
            conditional_providers: Default::default(),
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
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

    /// Returns whether the context should allow overriding existing providers.
    pub fn allow_override(&self) -> bool {
        self.allow_override
    }

    /// Returns whether the context should only eagerly create singleton instances.
    pub fn allow_only_singleton_eager_create(&self) -> bool {
        self.allow_only_singleton_eager_create
    }

    /// Returns whether the context should eagerly create instances.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns a reference to the singleton registry.
    pub fn singleton_registry(&self) -> &HashMap<Key, DynSingletonInstance> {
        self.singleton_registry.inner()
    }

    /// Returns a reference to the provider registry.
    pub fn provider_registry(&self) -> &HashMap<Key, DynProvider> {
        self.provider_registry.inner()
    }

    /// Returns a reference to the loaded modules.
    pub fn loaded_modules(&self) -> &Vec<Type> {
        &self.loaded_modules
    }

    /// Returns a reference to the conditional providers.
    pub fn conditional_providers(&self) -> &Vec<(bool, DynProvider)> {
        &self.conditional_providers
    }

    /// Returns a reference to the eager create functions.
    pub fn eager_create_functions(&self) -> &Vec<(Definition, EagerCreateFunction)> {
        &self.eager_create_functions
    }

    /// Returns a reference to the dependency chain.
    pub fn dependency_chain(&self) -> &Vec<Key> {
        &self.dependency_chain.stack
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
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
            self.loaded_modules.push(module.ty());
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
            self.loaded_modules.retain(|ty| ty != &module.ty());
            self.unload_providers(module.providers());
        });
    }

    /// Flush the context.
    ///
    /// This method has two purposes:
    ///
    /// 1. Evaluate the conditions of providers whose [`condition`](crate::Provider::condition) is `Some`.
    /// If the evaluation result is `true`, the provider will be loaded into the context,
    /// otherwise it will be removed from the context.
    ///
    /// 2. Construct instances that will be eagerly created.
    /// When a provider is loaded into the context,
    /// the `need_eager_create` is obtained by performing a logical OR operation on
    /// the [`eager_create`](crate::Provider::eager_create) value of the provider,
    /// the [`eager_create`](crate::ResolveModule::eager_create) value of the module to which the provider belongs,
    /// and the [`eager_create`](crate::Context::eager_create) value of the `Context`.
    /// Then, the `allow_eager_create` is obtained by evaluating
    /// the `Context`'s [`allow_only_singleton_eager_create`](crate::Context::allow_only_singleton_eager_create)
    /// and the provider's [`scope`](crate::Scope).
    /// If the result of the logical AND operation of `need_eager_create` and `allow_eager_create` is `true`,
    /// the provider's constructor will be pushed into a queue. When this method is called,
    /// the queue will be traversed and each constructor in the queue will be called to construct
    /// the instance of the provider.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, AutoRegisterModule, Context, Singleton, Transient};
    ///
    /// #[Transient(condition = |_| true)]
    /// struct A;
    ///
    /// #[derive(Clone)]
    /// #[Singleton(eager_create)]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    ///
    /// cx.load_modules(modules![AutoRegisterModule]);
    ///
    /// assert!(!cx.contains_provider::<A>());
    /// assert!(!cx.contains_singleton::<B>());
    ///
    /// cx.flush();
    ///
    /// // evaluate condition
    /// assert!(cx.contains_provider::<A>());
    /// // construct instance
    /// assert!(cx.contains_singleton::<B>());
    /// # }
    /// ```
    ///
    /// # Note
    ///
    /// This method needs to be called after the [`Context::load_modules`] method,
    /// but why not put the logic of this method in the `load_modules` method? Please see the example below:
    ///
    /// ```rust
    /// use rudi::{components, modules, Context, DynProvider, Module, Transient};
    ///
    /// fn a_condition(cx: &Context) -> bool {
    ///     cx.contains_provider::<B>()
    /// }
    ///
    /// #[Transient(condition = a_condition)]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// struct AModule;
    ///
    /// impl Module for AModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![A]
    ///     }
    /// }
    ///
    /// struct BModule;
    ///
    /// impl Module for BModule {
    ///     fn providers() -> Vec<DynProvider> {
    ///         components![B]
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let mut cx = Context::default();
    ///
    ///     // Method 1, call `load_modules` and then call `flush` immediately
    ///     cx.load_modules(modules![AModule]);
    ///     cx.flush();
    ///     cx.load_modules(modules![BModule]);
    ///     cx.flush();
    ///
    ///     // The evaluation result of `A`'s `condition` is `false`, so `A` will not be created
    ///     assert!(!cx.contains_provider::<A>());
    ///
    ///     let mut cx = Context::default();
    ///
    ///     // Method 2, call all `load_modules` first, then call `flush`
    ///     cx.load_modules(modules![AModule]);
    ///     cx.load_modules(modules![BModule]);
    ///     cx.flush();
    ///
    ///     // The evaluation result of `A`'s `condition` is `true`, so `A` will be created
    ///     assert!(cx.contains_provider::<A>());
    /// }
    /// ```
    #[track_caller]
    pub fn flush(&mut self) {
        self.create_eager_instances();

        self.evaluate_providers();
        self.create_eager_instances();
    }

    /// Async version of [`Context::flush`].
    ///
    /// If no provider in the context has an async constructor and that provider needs to be eagerly created,
    /// this method is the same as [`Context::flush`].
    ///
    /// See [`Context::flush`] for more details.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn flush_async(&mut self) {
        self.create_eager_instances_async().await;

        self.evaluate_providers();
        self.create_eager_instances_async().await;
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
        match self.inner_resolve(name.into(), Behaviour::CreateThenReturn) {
            Resolved::Ok(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingleton(_) | Resolved::NoReturn => unreachable!(),
        }
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
        self.inner_resolve(name.into(), Behaviour::CreateThenReturn)
            .ok()
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
        self.names::<T>()
            .into_iter()
            .filter_map(|name| self.inner_resolve(name, Behaviour::CreateThenReturn).ok())
            .collect()
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn just_create<T: 'static>(&mut self, name: Cow<'static, str>) {
        match self.inner_resolve::<T>(name, Behaviour::JustCreate) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::Ok(_) | Resolved::NotSingleton(_) => unreachable!(),
        }
    }

    /// Creates a singleton instance based on the given type and default name `""` but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a singleton.
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
    /// let mut cx = Context::auto_register();
    /// assert!(!cx.contains_singleton::<A>());
    /// cx.just_create_singleton::<A>();
    /// assert!(cx.contains_singleton::<A>());
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_singleton<T: 'static>(&mut self) {
        self.just_create_singleton_with_name::<T>("");
    }

    /// Creates a singleton instance based on the given type and name but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a singleton.
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
    /// let mut cx = Context::auto_register();
    /// assert!(!cx.contains_singleton_with_name::<A>("a"));
    /// cx.just_create_singleton_with_name::<A>("a");
    /// assert!(cx.contains_singleton_with_name::<A>("a"));
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_singleton_with_name<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingleton) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingleton(definition) => not_singleton_panic(definition),
            Resolved::Ok(_) => unreachable!(),
        }
    }

    /// Try to create a singleton instance based on the given type and default name `""` but does not return it.
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a singleton,
    /// this method will do nothing.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton]
    /// struct A;
    ///
    /// #[Transient]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    ///
    /// assert!(!cx.contains_singleton::<A>());
    /// assert!(!cx.contains_singleton::<B>());
    ///
    /// cx.try_create_singleton::<A>();
    /// cx.try_create_singleton::<B>();
    ///
    /// assert!(cx.contains_singleton::<A>());
    /// assert!(!cx.contains_singleton::<B>());
    /// # }
    /// ```
    #[track_caller]
    pub fn try_create_singleton<T: 'static>(&mut self) {
        self.try_create_singleton_with_name::<T>("");
    }

    /// Try to create a singleton instance based on the given type and name but does not return it.
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a singleton,
    /// this method will do nothing.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(name = "a")]
    /// struct A;
    ///
    /// #[Transient(name = "b")]
    /// struct B;
    ///
    /// # fn main() {
    /// let mut cx = Context::auto_register();
    ///
    /// assert!(!cx.contains_singleton_with_name::<A>("a"));
    /// assert!(!cx.contains_singleton_with_name::<B>("b"));
    ///
    /// cx.try_create_singleton_with_name::<A>("a");
    /// cx.try_create_singleton_with_name::<B>("b");
    ///
    /// assert!(cx.contains_singleton_with_name::<A>("a"));
    /// assert!(!cx.contains_singleton_with_name::<B>("b"));
    /// # }
    /// ```
    #[track_caller]
    pub fn try_create_singleton_with_name<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingleton) {
            Resolved::NoReturn | Resolved::NotFoundProvider(_) | Resolved::NotSingleton(_) => {}
            Resolved::Ok(_) => unreachable!(),
        }
    }

    /// Try to create singleton instances based on the given type but does not return them.
    ///
    /// If some providers are not singletons, this method will not create them.
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[Singleton]
    /// fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient]
    /// fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert!(!cx.contains_singleton::<i32>());
    ///     cx.try_create_singletons_by_type::<i32>();
    ///     assert_eq!(cx.get_singleton::<i32>(), &1);
    /// }
    /// ```
    #[track_caller]
    pub fn try_create_singletons_by_type<T: 'static>(&mut self) {
        self.names::<T>()
            .into_iter()
            .for_each(|name| self.try_create_singleton_with_name::<T>(name))
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
        match self
            .inner_resolve_async(name.into(), Behaviour::CreateThenReturn)
            .await
        {
            Resolved::Ok(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingleton(_) | Resolved::NoReturn => unreachable!(),
        }
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
        self.inner_resolve_async(name.into(), Behaviour::CreateThenReturn)
            .await
            .ok()
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
        let names = self.names::<T>();

        let mut instances = Vec::with_capacity(names.len());

        for name in names {
            if let Some(instance) = self
                .inner_resolve_async(name, Behaviour::CreateThenReturn)
                .await
                .ok()
            {
                instances.push(instance);
            }
        }

        instances
    }

    #[doc(hidden)]
    pub async fn just_create_async<T: 'static>(&mut self, name: Cow<'static, str>) {
        match self
            .inner_resolve_async::<T>(name, Behaviour::JustCreate)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::Ok(_) | Resolved::NotSingleton(_) => unreachable!(),
        }
    }

    /// Async version of [`Context::just_create_singleton`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a singleton.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async)]
    /// struct A;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert!(!cx.contains_singleton::<A>());
    ///     cx.just_create_singleton_async::<A>().await;
    ///     assert!(cx.contains_singleton::<A>());
    /// }
    /// ```
    pub async fn just_create_singleton_async<T: 'static>(&mut self) {
        self.just_create_singleton_with_name_async::<T>("").await;
    }

    /// Async version of [`Context::just_create_singleton_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a singleton.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async, name = "a")]
    /// struct A;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert!(!cx.contains_singleton_with_name::<A>("a"));
    ///     cx.just_create_singleton_with_name_async::<A>("a").await;
    ///     assert!(cx.contains_singleton_with_name::<A>("a"));
    /// }
    /// ```
    pub async fn just_create_singleton_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingleton)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingleton(definition) => not_singleton_panic(definition),
            Resolved::Ok(_) => unreachable!(),
        }
    }

    /// Async version of [`Context::try_create_singleton`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async)]
    /// struct A;
    ///
    /// #[Transient(async)]
    /// struct B;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///
    ///     assert!(!cx.contains_singleton::<A>());
    ///     assert!(!cx.contains_singleton::<B>());
    ///
    ///     cx.try_create_singleton_async::<A>().await;
    ///     cx.try_create_singleton_async::<B>().await;
    ///
    ///     assert!(cx.contains_singleton::<A>());
    ///     assert!(!cx.contains_singleton::<B>());
    /// }
    /// ```
    pub async fn try_create_singleton_async<T: 'static>(&mut self) {
        self.try_create_singleton_with_name_async::<T>("").await;
    }

    /// Async version of [`Context::try_create_singleton_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[derive(Clone)]
    /// #[Singleton(async, name = "a")]
    /// struct A;
    ///
    /// #[Transient(async, name = "b")]
    /// struct B;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///
    ///     assert!(!cx.contains_singleton_with_name::<A>("a"));
    ///     assert!(!cx.contains_singleton_with_name::<B>("b"));
    ///
    ///     cx.try_create_singleton_with_name_async::<A>("a").await;
    ///     cx.try_create_singleton_with_name_async::<B>("b").await;
    ///
    ///     assert!(cx.contains_singleton_with_name::<A>("a"));
    ///     assert!(!cx.contains_singleton_with_name::<B>("b"));
    /// }
    /// ```
    pub async fn try_create_singleton_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingleton)
            .await
        {
            Resolved::NoReturn | Resolved::NotFoundProvider(_) | Resolved::NotSingleton(_) => {}
            Resolved::Ok(_) => unreachable!(),
        }
    }

    /// Async version of [`Context::try_create_singletons_by_type`].
    ///
    /// # Panics
    ///
    /// - Panics if there is a provider that panics on construction.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton, Transient};
    ///
    /// #[Singleton]
    /// async fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient]
    /// async fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///     assert!(!cx.contains_singleton::<i32>());
    ///     cx.try_create_singletons_by_type_async::<i32>().await;
    ///     assert_eq!(cx.get_singleton::<i32>(), &1);
    /// }
    /// ```
    pub async fn try_create_singletons_by_type_async<T: 'static>(&mut self) {
        for name in self.names::<T>() {
            self.try_create_singleton_with_name_async::<T>(name).await;
        }
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

        self.provider_registry()
            .iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, provider)| provider.as_provider())
            .collect()
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

    /// Returns a reference to a singleton based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no singleton is registered for the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// let a = cx.get_singleton::<A>();
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_singleton<T: 'static>(&self) -> &T {
        self.get_singleton_with_name("")
    }

    /// Returns a reference to a singleton based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no singleton is registered for the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// let a = cx.get_singleton_with_name::<A>("a");
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_singleton_with_name<T: 'static>(&self, name: impl Into<Cow<'static, str>>) -> &T {
        let key = Key::new::<T>(name.into());
        self.singleton_registry
            .get_ref(&key)
            .unwrap_or_else(|| panic!("no singleton registered for: {:?}", key))
    }

    /// Returns an optional reference to a singleton based on the given type and default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create)]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.get_singleton_option::<A>().is_some());
    /// # }
    /// ```
    pub fn get_singleton_option<T: 'static>(&self) -> Option<&T> {
        self.get_singleton_option_with_name("")
    }

    /// Returns an optional reference to a singleton based on the given type and name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[derive(Clone, Debug)]
    /// #[Singleton(eager_create, name = "a")]
    /// struct A;
    ///
    /// # fn main() {
    /// let cx = Context::auto_register();
    /// assert!(cx.get_singleton_option_with_name::<A>("a").is_some());
    /// # }
    /// ```
    pub fn get_singleton_option_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&T> {
        let key = Key::new::<T>(name.into());
        self.singleton_registry.get_ref(&key)
    }

    /// Returns a collection of references to singletons based on the given type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, Singleton};
    ///
    /// #[Singleton(eager_create, name = "a")]
    /// fn A() -> i32 {
    ///     1
    /// }
    ///
    /// #[Singleton(eager_create, name = "b")]
    /// fn B() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let cx = Context::auto_register();
    ///     assert_eq!(
    ///         cx.get_singletons_by_type::<i32>().into_iter().sum::<i32>(),
    ///         3
    ///     );
    /// }
    /// ```
    pub fn get_singletons_by_type<T: 'static>(&self) -> Vec<&T> {
        let type_id = TypeId::of::<T>();

        self.singleton_registry()
            .iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, singleton)| singleton.as_singleton())
            .map(|singleton| singleton.get_ref())
            .collect()
    }
}

impl Context {
    #[track_caller]
    fn load_provider(&mut self, eager_create: bool, provider: DynProvider) {
        let definition = provider.definition();
        let need_eager_create = self.eager_create || eager_create || provider.eager_create();

        let allow_all_scope = !self.allow_only_singleton_eager_create;
        let allow_only_singleton_and_it_is_singleton =
            self.allow_only_singleton_eager_create && matches!(definition.scope, Scope::Singleton);

        let allow_eager_create = allow_all_scope || allow_only_singleton_and_it_is_singleton;

        if need_eager_create && allow_eager_create {
            self.eager_create_functions
                .push((definition.clone(), provider.eager_create_function()));
        }

        self.provider_registry.insert(provider, self.allow_override);
    }

    #[track_caller]
    fn load_providers(&mut self, eager_create: bool, providers: Vec<DynProvider>) {
        let Some(providers) = flatten(providers, DynProvider::binding_providers) else {
            return;
        };

        providers.into_iter().for_each(|provider| {
            if provider.condition().is_some() {
                self.conditional_providers.push((eager_create, provider));
                return;
            }

            self.load_provider(eager_create, provider);
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
    fn create_eager_instances(&mut self) {
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

    async fn create_eager_instances_async(&mut self) {
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

    #[track_caller]
    fn evaluate_providers(&mut self) {
        if self.conditional_providers.is_empty() {
            return;
        }

        self.conditional_providers.reverse();

        while let Some((eager_create, provider)) = self.conditional_providers.pop() {
            if !(provider.condition().unwrap())(self) {
                #[cfg(feature = "tracing")]
                tracing::warn!("(×) condition not met: {:?}", provider.definition());
                continue;
            }

            self.load_provider(eager_create, provider);
        }
    }

    fn before_resolve<T: 'static>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Result<Resolved<T>, Holder<'_, T>> {
        let key = Key::new::<T>(name);

        if self.singleton_registry.contains(&key) {
            return Ok(match behaviour {
                Behaviour::CreateThenReturn => {
                    Resolved::Ok(self.singleton_registry.get_owned::<T>(&key).unwrap())
                }
                Behaviour::JustCreate | Behaviour::JustCreateSingleton => Resolved::NoReturn,
            });
        }

        let Some(provider) = self.provider_registry.get::<T>(&key) else {
            return Ok(Resolved::NotFoundProvider(key));
        };

        let definition = provider.definition();

        match (behaviour, definition.scope) {
            (_, Scope::Singleton) => {}
            (Behaviour::JustCreateSingleton, /* not singleton */ _) => {
                return Ok(Resolved::NotSingleton(definition.clone()))
            }
            _ => {}
        }

        let constructor = provider.constructor();
        let clone_instance = provider.clone_instance();

        Err(Holder {
            key,
            constructor,
            clone_instance,
            definition,
        })
    }

    fn after_resolve<T: 'static>(
        &mut self,
        key: Key,
        behaviour: Behaviour,
        instance: T,
        clone_instance: Option<fn(&T) -> T>,
    ) -> Resolved<T> {
        if let Some(clone_instance) = clone_instance {
            match behaviour {
                Behaviour::CreateThenReturn => {
                    self.singleton_registry.insert(
                        key,
                        SingletonInstance::new(clone_instance(&instance), clone_instance).into(),
                    );
                }
                Behaviour::JustCreate | Behaviour::JustCreateSingleton => {
                    self.singleton_registry
                        .insert(key, SingletonInstance::new(instance, clone_instance).into());

                    return Resolved::NoReturn;
                }
            };
        }

        match behaviour {
            Behaviour::CreateThenReturn => Resolved::Ok(instance),
            Behaviour::JustCreate => Resolved::NoReturn,
            Behaviour::JustCreateSingleton => unreachable!(),
        }
    }

    #[track_caller]
    fn inner_resolve<T: 'static>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Resolved<T> {
        let Holder {
            key,
            constructor,
            clone_instance,
            definition,
        } = match self.before_resolve(name, behaviour) {
            Ok(o) => return o,
            Err(e) => e,
        };

        let instance = match constructor {
            Constructor::Async(_) => {
                panic!(
                    "unable to call an async constructor in a sync context for: {:?}

please check all the references to the above type, there are 3 scenarios that will be referenced:
1. use `Context::resolve_xxx::<Type>(cx)` to get instances of the type, change to `Context::resolve_xxx_async::<Type>(cx).await`.
2. use `yyy: Type` as a field of a struct, or a field of a variant of a enum, use `#[Singleton(async)]` or `#[Transient(async)]` on the struct or enum.
3. use `zzz: Type` as a argument of a function, add the `async` keyword to the function.
",
                    definition
                )
            }
            Constructor::Sync(constructor) => self.resolve_instance(key.clone(), constructor),
        };

        self.after_resolve(key, behaviour, instance, clone_instance)
    }

    async fn inner_resolve_async<T: 'static>(
        &mut self,
        name: Cow<'static, str>,
        behaviour: Behaviour,
    ) -> Resolved<T> {
        let Holder {
            key,
            constructor,
            clone_instance,
            ..
        } = match self.before_resolve(name, behaviour) {
            Ok(o) => return o,
            Err(e) => e,
        };

        let instance = {
            let key = key.clone();

            match constructor {
                Constructor::Async(constructor) => {
                    self.resolve_instance_async(key, constructor).await
                }
                Constructor::Sync(constructor) => self.resolve_instance(key, constructor),
            }
        };

        self.after_resolve(key, behaviour, instance, clone_instance)
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

    fn names<T: 'static>(&self) -> Vec<Cow<'static, str>> {
        let type_id = TypeId::of::<T>();

        self.provider_registry()
            .keys()
            .filter(|&key| key.ty.id == type_id)
            .map(|key| key.name.clone())
            .collect()
    }
}

#[derive(Clone, Copy)]
enum Behaviour {
    CreateThenReturn,
    JustCreate,
    JustCreateSingleton,
}

enum Resolved<T> {
    Ok(T),
    NotFoundProvider(Key),
    NotSingleton(Definition),
    NoReturn,
}

struct Holder<'a, T> {
    key: Key,
    constructor: Constructor<T>,
    clone_instance: Option<fn(&T) -> T>,
    definition: &'a Definition,
}

impl<T> Resolved<T> {
    fn ok(self) -> Option<T> {
        match self {
            Resolved::Ok(instance) => Some(instance),
            _ => None,
        }
    }
}

#[inline(always)]
fn no_provider_panic(key: Key) -> ! {
    panic!("no provider registered for: {:?}", key)
}

#[inline(always)]
fn not_singleton_panic(definition: Definition) -> ! {
    panic!("registered provider is not singleton for: {:?}", definition)
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
    singletons: Vec<DynSingletonInstance>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_singleton_eager_create: true,
            eager_create: Default::default(),
            providers: Default::default(),
            singletons: Default::default(),
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
        let provider = Provider::<T>::never_construct(name.into()).into();
        let singleton = SingletonInstance::new(instance, Clone::clone).into();

        self.providers.push(provider);
        self.singletons.push(singleton);

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
            singletons,
        } = self;

        let mut cx = Context {
            allow_override,
            allow_only_singleton_eager_create,
            eager_create,
            ..Default::default()
        };

        if !providers.is_empty() {
            providers
                .into_iter()
                .zip(singletons)
                .for_each(|(provider, singleton)| {
                    let key = provider.key().clone();
                    cx.provider_registry.insert(provider, allow_override);
                    cx.singleton_registry.insert(key, singleton);
                });
        }

        init(&mut cx);

        cx
    }

    /// Creates a new context with the given modules.
    ///
    /// # Panics
    ///
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
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
        cx.flush();
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider whose constructor is async and the provider will be eagerly created.
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
            cx.loaded_modules.push(module.ty());
            cx.load_providers(module.eager_create(), module.providers())
        });

        cx.flush();
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    pub async fn create_async(self, modules: Vec<ResolveModule>) -> Context {
        let mut cx = self.inner_create(|cx| cx.load_modules(modules));
        cx.flush_async().await;
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
    /// - Panics if there are multiple providers with the same key and the context's [`allow_override`](Context::allow_override) is false.
    /// - Panics if there is a provider that panics on construction.
    #[cfg_attr(docsrs, doc(cfg(feature = "auto-register")))]
    #[cfg(feature = "auto-register")]
    pub async fn auto_register_async(self) -> Context {
        use crate::AutoRegisterModule;

        let mut cx = self.inner_create(|cx| {
            let module = ResolveModule::new::<AutoRegisterModule>();
            cx.loaded_modules.push(module.ty());
            cx.load_providers(module.eager_create(), module.providers())
        });

        cx.flush_async().await;
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
