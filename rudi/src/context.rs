use std::{any::TypeId, borrow::Cow, collections::HashMap, rc::Rc};

use crate::{
    BoxFuture, Constructor, Definition, DynProvider, DynSingle, EagerCreateFunction, Key, Provider,
    ProviderRegistry, ResolveModule, Scope, Single, SingleRegistry, Type,
};

/// A context is a container for all the providers and instances.
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
/// use rudi::{components, modules, Context, DynProvider, Module, Transient};
///
/// #[Transient]
/// struct A;
///
/// struct Module1;
///
/// impl Module for Module1 {
///     fn providers() -> Vec<DynProvider> {
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
///     fn providers() -> Vec<DynProvider> {
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
    allow_only_single_eager_create: bool,

    eager_create: bool,

    single_registry: SingleRegistry,
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
            allow_only_single_eager_create: true,
            eager_create: Default::default(),
            single_registry: Default::default(),
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
    /// use rudi::{components, modules, Context, DynProvider, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<DynProvider> {
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
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    pub fn options() -> ContextOptions {
        ContextOptions::default()
    }

    /// Returns whether the context should allow overriding existing providers.
    pub fn allow_override(&self) -> bool {
        self.allow_override
    }

    /// Returns whether the context should only eagerly create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances.
    pub fn allow_only_single_eager_create(&self) -> bool {
        self.allow_only_single_eager_create
    }

    /// Returns whether the context should eagerly create instances.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns a reference to the single registry.
    pub fn single_registry(&self) -> &HashMap<Key, DynSingle> {
        self.single_registry.inner()
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

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::Context;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    /// cx.insert_singleton(42);
    /// assert_eq!(cx.get_single::<i32>(), &42);
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_singleton<T>(&mut self, instance: T)
    where
        T: 'static + Clone,
    {
        self.insert_singleton_with_name(instance, "");
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with name.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::Context;
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    ///
    /// cx.insert_singleton_with_name(1, "one");
    /// cx.insert_singleton_with_name(2, "two");
    ///
    /// assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
    /// assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_singleton_with_name<T, N>(&mut self, instance: T, name: N)
    where
        T: 'static + Clone,
        N: Into<Cow<'static, str>>,
    {
        let provider: DynProvider =
            Provider::<T>::never_construct(name.into(), Scope::Singleton).into();
        let single = Single::new(instance, Some(Clone::clone)).into();

        let key = provider.key().clone();
        self.provider_registry.insert(provider, self.allow_override);
        self.single_registry.insert(key, single);
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::Context;
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    /// cx.insert_single_owner(NotClone(42));
    /// assert_eq!(cx.get_single::<NotClone>(), &NotClone(42));
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_single_owner<T>(&mut self, instance: T)
    where
        T: 'static,
    {
        self.insert_single_owner_with_name(instance, "");
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with name.
    ///
    /// # Panics
    ///
    /// - Panics if a `Provider<T>` with the same name as the inserted instance exists in the `Context` and the context's [`allow_override`](Context::allow_override) is false.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::Context;
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let mut cx = Context::default();
    ///
    /// cx.insert_single_owner_with_name(NotClone(1), "one");
    /// cx.insert_single_owner_with_name(NotClone(2), "two");
    ///
    /// assert_eq!(cx.get_single_with_name::<NotClone>("one"), &NotClone(1));
    /// assert_eq!(cx.get_single_with_name::<NotClone>("two"), &NotClone(2));
    /// # }
    /// ```
    #[track_caller]
    pub fn insert_single_owner_with_name<T, N>(&mut self, instance: T, name: N)
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let provider: DynProvider =
            Provider::<T>::never_construct(name.into(), Scope::SingleOwner).into();
        let single = Single::new(instance, None).into();

        let key = provider.key().clone();
        self.provider_registry.insert(provider, self.allow_override);
        self.single_registry.insert(key, single);
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
        if modules.is_empty() {
            return;
        }

        let modules = flatten(modules, ResolveModule::submodules);

        modules.into_iter().for_each(|module| {
            self.loaded_modules.push(module.ty());
            self.load_providers(module.eager_create(), module.providers());
        });
    }

    /// Unload the given modules.
    ///
    /// This method will convert the given module into a collection of providers like
    /// the [`Context::load_modules`] method, and then remove all providers in the context
    /// that are equal to the providers in the collection and their possible instances.
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
        if modules.is_empty() {
            return;
        }

        let modules = flatten(modules, ResolveModule::submodules);

        modules.into_iter().for_each(|module| {
            self.loaded_modules.retain(|ty| ty != &module.ty());
            self.unload_providers(module.providers());
        });
    }

    /// Flush the context.
    ///
    /// This method has two purposes:
    ///
    /// 1. Evaluate the condition of providers whose [`condition`](crate::Provider::condition) is `Some`.
    ///
    ///    If the evaluation result is `true`, the provider will be loaded into the context,
    ///    otherwise it will be removed from the context.
    ///
    /// 2. Construct instances that will be eagerly created.
    ///
    ///    Whether an instance need to be created eagerly depends on
    ///    the [`eager_create`](crate::Provider::eager_create) field of the Provider that defines it,
    ///    the [`eager_create`](crate::ResolveModule::eager_create) field of the Module to which this Provider belongs,
    ///    and the [`eager_create`](crate::Context::eager_create) field of the Context to which this Module belongs.
    ///    As long as one of these is true, the instance need to be created eagerly.
    ///
    ///    Whether an instance is allowed to be created eagerly depends on
    ///    the [`scope`](crate::Definition::scope) field in the [`definition`](crate::Provider::definition) field of the Provider that defines it,
    ///    and the [`allow_only_single_eager_create`](crate::Context::allow_only_single_eager_create) field of the Context to which this Provider belongs.
    ///    If `allow_only_single_eager_create` is false, or `allow_only_single_eager_create` is true and `scope` is [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    ///    the instance is allowed to be created eagerly.
    ///
    ///    When an instance need to be created eagerly and is allowed to be created eagerly, it will be created eagerly.
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
    /// assert!(!cx.contains_single::<B>());
    ///
    /// cx.flush();
    ///
    /// // evaluate condition
    /// assert!(cx.contains_provider::<A>());
    /// // construct instance
    /// assert!(cx.contains_single::<B>());
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

    /// Returns a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
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

    /// Returns a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
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
        match self.inner_resolve(name.into(), Behaviour::CreateThenReturnSingletonOrTransient) {
            Resolved::SingletonOrTransient(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrTransient(definition) => {
                not_singleton_or_transient_panic(definition)
            }
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Returns an optional [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and default name `""`.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient),
    /// this method will return `None`, otherwise it will return `Some`.
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

    /// Returns an optional [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient) instance based on the given type and name.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and name, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient),
    /// this method will return `None`, otherwise it will return `Some`.
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
        match self.inner_resolve(name.into(), Behaviour::CreateThenReturnSingletonOrTransient) {
            Resolved::SingletonOrTransient(instance) => Some(instance),
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrTransient(_) => None,
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
    }

    /// Returns a collection of [`Singleton`](crate::Scope::Singleton) and [`Transient`](crate::Scope::Transient) instances of the given type.
    ///
    /// # Note
    ///
    /// This method will return a collection of [`Singleton`](crate::Scope::Singleton) and [`Transient`](crate::Scope::Transient),
    /// if some providers are [`SingleOwner`](crate::Scope::SingleOwner), they will not be contained in the collection.
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
            .filter_map(|name| self.resolve_option_with_name(name))
            .collect()
    }

    #[doc(hidden)]
    #[track_caller]
    pub fn just_create<T: 'static>(&mut self, name: Cow<'static, str>) {
        match self.inner_resolve::<T>(name, Behaviour::JustCreateAllScopeForEagerCreate) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::SingletonOrTransient(_)
            | Resolved::NotSingletonOrTransient(_)
            | Resolved::NotSingletonOrSingleOwner(_) => {
                unreachable!()
            }
        }
    }

    /// Creates a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""` but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
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
    /// assert!(!cx.contains_single::<A>());
    /// cx.just_create_single::<A>();
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_single<T: 'static>(&mut self) {
        self.just_create_single_with_name::<T>("");
    }

    /// Creates a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name but does not return it.
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider whose constructor is async.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
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
    /// assert!(!cx.contains_single_with_name::<A>("a"));
    /// cx.just_create_single_with_name::<A>("a");
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// # }
    /// ```
    #[track_caller]
    pub fn just_create_single_with_name<T: 'static>(&mut self, name: impl Into<Cow<'static, str>>) {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner) {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrSingleOwner(definition) => {
                not_singleton_or_single_owner_panic(definition)
            }
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Try to create a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""` but does not return it.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// this method will return `false`, otherwise it will return `true`.
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
    /// assert!(!cx.contains_single::<A>());
    /// assert!(!cx.contains_single::<B>());
    ///
    /// assert!(cx.try_just_create_single::<A>());
    /// assert!(!cx.try_just_create_single::<B>());
    ///
    /// assert!(cx.contains_single::<A>());
    /// assert!(!cx.contains_single::<B>());
    /// # }
    /// ```
    #[track_caller]
    pub fn try_just_create_single<T: 'static>(&mut self) -> bool {
        self.try_just_create_single_with_name::<T>("")
    }

    /// Try to create a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name but does not return it.
    ///
    /// # Note
    ///
    /// If no provider is registered for the given type and default name `""`, or the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// this method will return `false`, otherwise it will return `true`.
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
    /// assert!(!cx.contains_single_with_name::<A>("a"));
    /// assert!(!cx.contains_single_with_name::<B>("b"));
    ///
    /// assert!(cx.try_just_create_single_with_name::<A>("a"));
    /// assert!(!cx.try_just_create_single_with_name::<B>("b"));
    ///
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// assert!(!cx.contains_single_with_name::<B>("b"));
    /// # }
    /// ```
    #[track_caller]
    pub fn try_just_create_single_with_name<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        match self.inner_resolve::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner) {
            Resolved::NoReturn => true,
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrSingleOwner(_) => false,
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Try to create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances based on the given type but does not return them.
    ///
    /// # Note
    ///
    /// This method will return a collection of booleans, if a provider is a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner),
    /// the corresponding boolean value will be `true`, otherwise it will be `false`.
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
    /// #[Singleton(name = "one")]
    /// fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "two")]
    /// fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// fn main() {
    ///     let mut cx = Context::auto_register();
    ///
    ///     assert!(!cx.contains_single::<i32>());
    ///
    ///     let results = cx.try_just_create_singles_by_type::<i32>();
    ///
    ///     assert!(results.contains(&true));
    ///     assert!(results.contains(&false));
    ///
    ///     assert_eq!(cx.get_singles_by_type::<i32>(), vec![&1]);
    /// }
    /// ```
    #[track_caller]
    pub fn try_just_create_singles_by_type<T: 'static>(&mut self) -> Vec<bool> {
        self.names::<T>()
            .into_iter()
            .map(|name| self.try_just_create_single_with_name::<T>(name))
            .collect()
    }

    /// Async version of [`Context::resolve`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
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
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`Transient`](crate::Scope::Transient).
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
            .inner_resolve_async(name.into(), Behaviour::CreateThenReturnSingletonOrTransient)
            .await
        {
            Resolved::SingletonOrTransient(instance) => instance,
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrTransient(definition) => {
                not_singleton_or_transient_panic(definition)
            }
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
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
        match self
            .inner_resolve_async(name.into(), Behaviour::CreateThenReturnSingletonOrTransient)
            .await
        {
            Resolved::SingletonOrTransient(instance) => Some(instance),
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrTransient(_) => None,
            Resolved::NotSingletonOrSingleOwner(_) | Resolved::NoReturn => unreachable!(),
        }
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
            if let Some(instance) = self.resolve_option_with_name_async(name).await {
                instances.push(instance);
            }
        }

        instances
    }

    #[doc(hidden)]
    pub async fn just_create_async<T: 'static>(&mut self, name: Cow<'static, str>) {
        match self
            .inner_resolve_async::<T>(name, Behaviour::JustCreateAllScopeForEagerCreate)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::SingletonOrTransient(_)
            | Resolved::NotSingletonOrTransient(_)
            | Resolved::NotSingletonOrSingleOwner(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`Context::just_create_single`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and default name `""`.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
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
    ///     assert!(!cx.contains_single::<A>());
    ///     cx.just_create_single_async::<A>().await;
    ///     assert!(cx.contains_single::<A>());
    /// }
    /// ```
    pub async fn just_create_single_async<T: 'static>(&mut self) {
        self.just_create_single_with_name_async::<T>("").await;
    }

    /// Async version of [`Context::just_create_single_with_name`].
    ///
    /// # Panics
    ///
    /// - Panics if no provider is registered for the given type and name.
    /// - Panics if there is a provider that panics on construction.
    /// - Panics if the provider is not a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner).
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
    ///     assert!(!cx.contains_single_with_name::<A>("a"));
    ///     cx.just_create_single_with_name_async::<A>("a").await;
    ///     assert!(cx.contains_single_with_name::<A>("a"));
    /// }
    /// ```
    pub async fn just_create_single_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner)
            .await
        {
            Resolved::NoReturn => {}
            Resolved::NotFoundProvider(key) => no_provider_panic(key),
            Resolved::NotSingletonOrSingleOwner(definition) => {
                not_singleton_or_single_owner_panic(definition)
            }
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`Context::try_just_create_single`].
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
    ///     assert!(!cx.contains_single::<A>());
    ///     assert!(!cx.contains_single::<B>());
    ///
    ///     assert!(cx.try_just_create_single_async::<A>().await);
    ///     assert!(!cx.try_just_create_single_async::<B>().await);
    ///
    ///     assert!(cx.contains_single::<A>());
    ///     assert!(!cx.contains_single::<B>());
    /// }
    /// ```
    pub async fn try_just_create_single_async<T: 'static>(&mut self) -> bool {
        self.try_just_create_single_with_name_async::<T>("").await
    }

    /// Async version of [`Context::try_just_create_single_with_name`].
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
    ///     assert!(!cx.contains_single_with_name::<A>("a"));
    ///     assert!(!cx.contains_single_with_name::<B>("b"));
    ///
    ///     assert!(cx.try_just_create_single_with_name_async::<A>("a").await);
    ///     assert!(!cx.try_just_create_single_with_name_async::<B>("b").await);
    ///
    ///     assert!(cx.contains_single_with_name::<A>("a"));
    ///     assert!(!cx.contains_single_with_name::<B>("b"));
    /// }
    /// ```
    pub async fn try_just_create_single_with_name_async<T: 'static>(
        &mut self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        match self
            .inner_resolve_async::<T>(name.into(), Behaviour::JustCreateSingletonOrSingleOwner)
            .await
        {
            Resolved::NoReturn => true,
            Resolved::NotFoundProvider(_) | Resolved::NotSingletonOrSingleOwner(_) => false,
            Resolved::SingletonOrTransient(_) | Resolved::NotSingletonOrTransient(_) => {
                unreachable!()
            }
        }
    }

    /// Async version of [`Context::try_just_create_singles_by_type`].
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
    /// #[Singleton(name = "one")]
    /// async fn One() -> i32 {
    ///     1
    /// }
    ///
    /// #[Transient(name = "two")]
    /// async fn Two() -> i32 {
    ///     2
    /// }
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut cx = Context::auto_register();
    ///
    ///     assert!(!cx.contains_single::<i32>());
    ///
    ///     let results = cx.try_just_create_singles_by_type_async::<i32>().await;
    ///
    ///     assert!(results.contains(&true));
    ///     assert!(results.contains(&false));
    ///
    ///     assert_eq!(cx.get_singles_by_type::<i32>(), vec![&1]);
    /// }
    /// ```
    pub async fn try_just_create_singles_by_type_async<T: 'static>(&mut self) -> Vec<bool> {
        let names = self.names::<T>();
        let mut results = Vec::with_capacity(names.len());

        for name in names {
            let result = self.try_just_create_single_with_name_async::<T>(name).await;
            results.push(result);
        }

        results
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

    /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and default name `""`.
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
    /// assert!(cx.contains_single::<A>());
    /// # }
    /// ```
    pub fn contains_single<T: 'static>(&self) -> bool {
        self.contains_single_with_name::<T>("")
    }

    /// Returns true if the context contains a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance for the specified type and name.
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
    /// assert!(cx.contains_single_with_name::<A>("a"));
    /// # }
    /// ```
    pub fn contains_single_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> bool {
        let key = Key::new::<T>(name.into());
        self.single_registry.contains(&key)
    }

    /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
    ///
    /// # Panics
    ///
    /// - Panics if no single instance is registered for the given type and default name `""`.
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
    /// let a = cx.get_single::<A>();
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_single<T: 'static>(&self) -> &T {
        self.get_single_with_name("")
    }

    /// Returns a reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
    ///
    /// # Panics
    ///
    /// - Panics if no single instance is registered for the given type and name.
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
    /// let a = cx.get_single_with_name::<A>("a");
    /// assert_eq!(format!("{:?}", a), "A");
    /// # }
    /// ```
    #[track_caller]
    pub fn get_single_with_name<T: 'static>(&self, name: impl Into<Cow<'static, str>>) -> &T {
        let key = Key::new::<T>(name.into());
        self.single_registry
            .get_ref(&key)
            .unwrap_or_else(|| panic!("no instance registered for: {:?}", key))
    }

    /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and default name `""`.
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
    /// assert!(cx.get_single_option::<A>().is_some());
    /// # }
    /// ```
    pub fn get_single_option<T: 'static>(&self) -> Option<&T> {
        self.get_single_option_with_name("")
    }

    /// Returns an optional reference to a [`Singleton`](crate::Scope::Singleton) or [`SingleOwner`](crate::Scope::SingleOwner) instance based on the given type and name.
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
    /// assert!(cx.get_single_option_with_name::<A>("a").is_some());
    /// # }
    /// ```
    pub fn get_single_option_with_name<T: 'static>(
        &self,
        name: impl Into<Cow<'static, str>>,
    ) -> Option<&T> {
        let key = Key::new::<T>(name.into());
        self.single_registry.get_ref(&key)
    }

    /// Returns a collection of references to [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances based on the given type.
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
    ///     assert_eq!(cx.get_singles_by_type::<i32>().into_iter().sum::<i32>(), 3);
    /// }
    /// ```
    pub fn get_singles_by_type<T: 'static>(&self) -> Vec<&T> {
        let type_id = TypeId::of::<T>();

        self.single_registry()
            .iter()
            .filter(|(key, _)| key.ty.id == type_id)
            .filter_map(|(_, instance)| instance.as_single())
            .map(|instance| instance.get_ref())
            .collect()
    }
}

impl Context {
    #[track_caller]
    fn load_provider(&mut self, eager_create: bool, provider: DynProvider) {
        let definition = provider.definition();
        let need_eager_create = self.eager_create || eager_create || provider.eager_create();

        let allow_all_scope = !self.allow_only_single_eager_create;

        let allow_only_single_and_it_is_single = matches!(
            (self.allow_only_single_eager_create, definition.scope),
            (true, Scope::Singleton) | (true, Scope::SingleOwner)
        );

        let allow_eager_create = allow_all_scope || allow_only_single_and_it_is_single;

        if need_eager_create && allow_eager_create {
            self.eager_create_functions
                .push((definition.clone(), provider.eager_create_function()));
        }

        self.provider_registry.insert(provider, self.allow_override);
    }

    #[track_caller]
    fn load_providers(&mut self, eager_create: bool, providers: Vec<DynProvider>) {
        if providers.is_empty() {
            return;
        }

        let providers = flatten(providers, DynProvider::binding_providers);

        providers.into_iter().for_each(|provider| {
            if provider.condition().is_some() {
                self.conditional_providers.push((eager_create, provider));
                return;
            }

            self.load_provider(eager_create, provider);
        });
    }

    fn unload_providers(&mut self, providers: Vec<DynProvider>) {
        if providers.is_empty() {
            return;
        }

        let providers = flatten(providers, DynProvider::binding_providers);

        providers.into_iter().for_each(|provider| {
            let key = provider.key();
            self.provider_registry.remove(key);
            self.single_registry.remove(key);
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
                EagerCreateFunction::None => unreachable!(),
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
                EagerCreateFunction::None => unreachable!(),
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

        let Some(provider) = self.provider_registry.get::<T>(&key) else {
            return Ok(Resolved::NotFoundProvider(key));
        };

        let definition = provider.definition();

        if self.single_registry.contains(&key) {
            return Ok(match behaviour {
                Behaviour::CreateThenReturnSingletonOrTransient => {
                    match self.single_registry.get_owned::<T>(&key) {
                        Some(instance) => Resolved::SingletonOrTransient(instance),
                        None => Resolved::NotSingletonOrTransient(definition.clone()),
                    }
                }
                Behaviour::JustCreateAllScopeForEagerCreate
                | Behaviour::JustCreateSingletonOrSingleOwner => Resolved::NoReturn,
            });
        }

        match (definition.scope, behaviour) {
            (Scope::Transient, Behaviour::JustCreateSingletonOrSingleOwner) => {
                return Ok(Resolved::NotSingletonOrSingleOwner(definition.clone()))
            }
            (Scope::SingleOwner, Behaviour::CreateThenReturnSingletonOrTransient) => {
                return Ok(Resolved::NotSingletonOrTransient(definition.clone()))
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
        scope: Scope,
        instance: T,
        clone_instance: Option<fn(&T) -> T>,
    ) -> Resolved<T> {
        match (scope, behaviour) {
            // Singleton
            (Scope::Singleton, Behaviour::CreateThenReturnSingletonOrTransient) => {
                self.single_registry.insert(
                    key,
                    Single::new((clone_instance.unwrap())(&instance), clone_instance).into(),
                );

                Resolved::SingletonOrTransient(instance)
            }
            (Scope::Singleton, Behaviour::JustCreateAllScopeForEagerCreate)
            | (Scope::Singleton, Behaviour::JustCreateSingletonOrSingleOwner) => {
                self.single_registry
                    .insert(key, Single::new(instance, clone_instance).into());

                Resolved::NoReturn
            }
            // Transient
            (Scope::Transient, Behaviour::CreateThenReturnSingletonOrTransient) => {
                Resolved::SingletonOrTransient(instance)
            }
            (Scope::Transient, Behaviour::JustCreateAllScopeForEagerCreate) => Resolved::NoReturn,
            (Scope::Transient, Behaviour::JustCreateSingletonOrSingleOwner) => unreachable!(),
            // SingleOwner
            (Scope::SingleOwner, Behaviour::CreateThenReturnSingletonOrTransient) => unreachable!(),
            (Scope::SingleOwner, Behaviour::JustCreateAllScopeForEagerCreate)
            | (Scope::SingleOwner, Behaviour::JustCreateSingletonOrSingleOwner) => {
                self.single_registry
                    .insert(key, Single::new(instance, None).into());

                Resolved::NoReturn
            }
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

        let scope = definition.scope;

        let instance = match constructor {
            Constructor::Async(_) => {
                panic!(
                    "unable to call an async constructor in a sync context for: {:?}

please check all the references to the above type, there are 3 scenarios that will be referenced:
1. use `Context::resolve_xxx::<Type>(cx)` to get instances of the type, change to `Context::resolve_xxx_async::<Type>(cx).await`.
2. use `yyy: Type` as a field of a struct, or a field of a variant of a enum, use `#[Singleton(async)]`, `#[Transient(async)]` or `#[SingleOwner(async)]` on the struct or enum.
3. use `zzz: Type` as a argument of a function, add the `async` keyword to the function.
",
                    definition
                )
            }
            Constructor::Sync(constructor) => self.resolve_instance(key.clone(), constructor),
            Constructor::None => unreachable!(),
        };

        self.after_resolve(key, behaviour, scope, instance, clone_instance)
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
            definition,
        } = match self.before_resolve(name, behaviour) {
            Ok(o) => return o,
            Err(e) => e,
        };

        let scope = definition.scope;

        let instance = {
            let key = key.clone();

            match constructor {
                Constructor::Async(constructor) => {
                    self.resolve_instance_async(key, constructor).await
                }
                Constructor::Sync(constructor) => self.resolve_instance(key, constructor),
                Constructor::None => unreachable!(),
            }
        };

        self.after_resolve(key, behaviour, scope, instance, clone_instance)
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
    CreateThenReturnSingletonOrTransient,
    JustCreateAllScopeForEagerCreate,
    JustCreateSingletonOrSingleOwner,
}

enum Resolved<T> {
    NotFoundProvider(Key),

    SingletonOrTransient(T),
    NotSingletonOrTransient(Definition),

    NoReturn,

    NotSingletonOrSingleOwner(Definition),
}

struct Holder<'a, T> {
    key: Key,
    constructor: Constructor<T>,
    clone_instance: Option<fn(&T) -> T>,
    definition: &'a Definition,
}

#[inline(always)]
fn no_provider_panic(key: Key) -> ! {
    panic!("no provider registered for: {:?}", key)
}

#[inline(always)]
fn not_singleton_or_single_owner_panic(definition: Definition) -> ! {
    panic!(
        "registered provider is not `Singleton` or `SingleOwner` for: {:?}",
        definition
    )
}

#[inline(always)]
fn not_singleton_or_transient_panic(definition: Definition) -> ! {
    panic!(
        "registered provider is not `Singleton` or `Transient` for: {:?}",
        definition
    )
}

fn flatten<T, F>(mut unresolved: Vec<T>, get_sublist: F) -> Vec<T>
where
    F: Fn(&mut T) -> Option<Vec<T>>,
{
    debug_assert!(!unresolved.is_empty());

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

    resolved
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
/// use rudi::{modules, Context, ContextOptions, DynProvider, Module};
///
/// struct MyModule;
///
/// impl Module for MyModule {
///     fn providers() -> Vec<DynProvider> {
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
///     .allow_only_single_eager_create(true)
///     .eager_create(false)
///     .singleton(42)
///     .singleton_with_name("Hello", "str_1")
///     .singleton_with_name("World", "str_2")
///     .create(modules![AutoRegisterModule]);
/// # }
/// ```
///
/// [`AutoRegisterModule`]: crate::AutoRegisterModule
pub struct ContextOptions {
    allow_override: bool,
    allow_only_single_eager_create: bool,
    eager_create: bool,
    providers: Vec<DynProvider>,
    singles: Vec<DynSingle>,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            allow_override: true,
            allow_only_single_eager_create: true,
            eager_create: Default::default(),
            providers: Default::default(),
            singles: Default::default(),
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

    /// Sets the option for whether the context should only eagerly create [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) instances.
    ///
    /// This option, when true, will only eagerly create instances for [`Singleton`](crate::Scope::Singleton) and [`SingleOwner`](crate::Scope::SingleOwner) providers.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{Context, ContextOptions};
    ///
    /// # fn main() {
    /// let _cx: Context = ContextOptions::default()
    ///     .allow_only_single_eager_create(false)
    ///     .auto_register();
    /// # }
    /// ```
    pub fn allow_only_single_eager_create(mut self, allow_only_single_eager_create: bool) -> Self {
        self.allow_only_single_eager_create = allow_only_single_eager_create;
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

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, Context, ContextOptions};
    ///
    /// # fn main() {
    /// let cx: Context = ContextOptions::default().singleton(42).create(modules![]);
    /// assert_eq!(cx.get_single::<i32>(), &42);
    /// # }
    /// ```
    pub fn singleton<T>(self, instance: T) -> Self
    where
        T: 'static + Clone,
    {
        self.singleton_with_name(instance, "")
    }

    /// Appends a standalone [`Singleton`](crate::Scope::Singleton) instance to the context with name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, Context, ContextOptions};
    ///
    /// # fn main() {
    /// let cx: Context = ContextOptions::default()
    ///     .singleton_with_name(1, "one")
    ///     .singleton_with_name(2, "two")
    ///     .create(modules![]);
    ///
    /// assert_eq!(cx.get_single_with_name::<i32>("one"), &1);
    /// assert_eq!(cx.get_single_with_name::<i32>("two"), &2);
    /// # }
    /// ```
    pub fn singleton_with_name<T, N>(mut self, instance: T, name: N) -> Self
    where
        T: 'static + Clone,
        N: Into<Cow<'static, str>>,
    {
        let provider = Provider::<T>::never_construct(name.into(), Scope::Singleton).into();
        let single = Single::new(instance, Some(Clone::clone)).into();

        self.providers.push(provider);
        self.singles.push(single);

        self
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with default name `""`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, Context, ContextOptions};
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let cx: Context = ContextOptions::default()
    ///     .single_owner(NotClone(42))
    ///     .create(modules![]);
    /// assert_eq!(cx.get_single::<NotClone>(), &NotClone(42));
    /// # }
    /// ```
    pub fn single_owner<T>(self, instance: T) -> Self
    where
        T: 'static,
    {
        self.single_owner_with_name(instance, "")
    }

    /// Appends a standalone [`SingleOwner`](crate::Scope::SingleOwner) instance to the context with name.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rudi::{modules, Context, ContextOptions};
    ///
    /// #[derive(PartialEq, Eq, Debug)]
    /// struct NotClone(i32);
    ///
    /// # fn main() {
    /// let cx: Context = ContextOptions::default()
    ///     .single_owner_with_name(NotClone(1), "one")
    ///     .single_owner_with_name(NotClone(2), "two")
    ///     .create(modules![]);
    ///
    /// assert_eq!(cx.get_single_with_name::<NotClone>("one"), &NotClone(1));
    /// assert_eq!(cx.get_single_with_name::<NotClone>("two"), &NotClone(2));
    /// # }
    /// ```
    pub fn single_owner_with_name<T, N>(mut self, instance: T, name: N) -> Self
    where
        T: 'static,
        N: Into<Cow<'static, str>>,
    {
        let provider = Provider::<T>::never_construct(name.into(), Scope::SingleOwner).into();
        let single = Single::new(instance, None).into();

        self.providers.push(provider);
        self.singles.push(single);

        self
    }

    #[track_caller]
    fn inner_create<F>(self, init: F) -> Context
    where
        F: FnOnce(&mut Context),
    {
        let ContextOptions {
            allow_override,
            allow_only_single_eager_create,
            eager_create,
            providers,
            singles,
        } = self;

        let mut cx = Context {
            allow_override,
            allow_only_single_eager_create,
            eager_create,
            ..Default::default()
        };

        if !providers.is_empty() {
            providers
                .into_iter()
                .zip(singles)
                .for_each(|(provider, single)| {
                    let key = provider.key().clone();
                    cx.provider_registry.insert(provider, allow_override);
                    cx.single_registry.insert(key, single);
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
    /// use rudi::{components, modules, Context, ContextOptions, DynProvider, Module, Transient};
    ///
    /// #[Transient]
    /// struct A;
    ///
    /// struct MyModule;
    ///
    /// impl Module for MyModule {
    ///     fn providers() -> Vec<DynProvider> {
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
