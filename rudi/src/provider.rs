use std::{any::Any, borrow::Cow, rc::Rc};

use crate::{BoxFuture, Color, Context, Definition, FutureExt, Key, Scope};

/// A trait for giving a type a default [`Provider`].
///
/// Define this trait so that the purpose is not to be implemented manually,
/// but to use the [`#[Singleton]`](crate::Singleton), [`#[Transient]`](crate::Transient) or [`#[SingleOwner]`](crate::SingleOwner) attribute macros to generate the implementation.
///
/// # Example
///
/// ```rust
/// use rudi::{DefaultProvider, Provider, Singleton, Transient};
///
/// #[Transient]
/// struct A;
///
/// #[Singleton]
/// fn Number() -> i32 {
///     42
/// }
///
/// fn main() {
///     let _: Provider<A> = <A as DefaultProvider>::provider();
///     let _: Provider<i32> = <Number as DefaultProvider>::provider();
/// }
/// ```
pub trait DefaultProvider {
    /// The generic of the [`Provider`].
    type Type;

    /// Returns a default [`Provider`] for the implementation.
    fn provider() -> Provider<Self::Type>;
}

pub(crate) enum Constructor<T> {
    #[allow(clippy::type_complexity)]
    Async(Rc<dyn for<'a> Fn(&'a mut Context) -> BoxFuture<'a, T>>),
    Sync(Rc<dyn Fn(&mut Context) -> T>),
    None,
}

impl<T> Clone for Constructor<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Async(c) => Self::Async(Rc::clone(c)),
            Self::Sync(c) => Self::Sync(Rc::clone(c)),
            Self::None => Self::None,
        }
    }
}

/// Represents the eager create function.
#[derive(Clone)]
pub enum EagerCreateFunction {
    /// async eager create function.
    Async(for<'a> fn(&'a mut Context, Cow<'static, str>) -> BoxFuture<'a, ()>),
    /// sync eager create function.
    Sync(fn(&mut Context, Cow<'static, str>)),
    /// no eager create function.
    None,
}

/// Represents the provider of an instance of type `T`.
///
/// This struct is just a generic, intermediate representation of `Provider`,
/// there is no pub method to direct create this struct,
/// Please use the following functions or attribute macros to create the various `Provider` types that implement `Into<Provider>`:
/// - functions
///   - [`singleton`](crate::singleton)
///   - [`transient`](crate::transient)
///   - [`single_owner`](crate::single_owner)
///   - [`singleton_async`](crate::singleton_async)
///   - [`transient_async`](crate::transient_async)
///   - [`single_owner_async`](crate::single_owner_async)
/// - attribute macros
///   - [`Singleton`](crate::Singleton)
///   - [`Transient`](crate::Transient)
///   - [`SingleOwner`](crate::SingleOwner)
pub struct Provider<T> {
    definition: Definition,
    eager_create: bool,
    condition: Option<fn(&Context) -> bool>,
    constructor: Constructor<T>,
    clone_instance: Option<fn(&T) -> T>,
    eager_create_function: EagerCreateFunction,
    binding_providers: Option<Vec<DynProvider>>,
    binding_definitions: Option<Vec<Definition>>,
}

impl<T> Provider<T> {
    /// Returns the [`Definition`] of the provider.
    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// Returns whether the provider is eager create.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns definitions of the binding providers.
    pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
        self.binding_definitions.as_ref()
    }

    /// Returns an option of the condition function.
    pub fn condition(&self) -> Option<fn(&Context) -> bool> {
        self.condition
    }

    pub(crate) fn constructor(&self) -> Constructor<T> {
        self.constructor.clone()
    }

    pub(crate) fn clone_instance(&self) -> Option<fn(&T) -> T> {
        self.clone_instance
    }
}

impl<T: 'static> Provider<T> {
    pub(crate) fn with_name(
        name: Cow<'static, str>,
        scope: Scope,
        eager_create: bool,
        condition: Option<fn(&Context) -> bool>,
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        let definition = Definition::new::<T>(
            name,
            scope,
            Some(match constructor {
                Constructor::Async(_) => Color::Async,
                Constructor::Sync(_) => Color::Sync,
                Constructor::None => unreachable!(),
            }),
            condition.is_some(),
        );

        Provider {
            definition,
            eager_create,
            condition,
            constructor,
            clone_instance,
            eager_create_function,
            binding_providers: None,
            binding_definitions: None,
        }
    }

    pub(crate) fn with_definition(
        definition: Definition,
        eager_create: bool,
        condition: Option<fn(&Context) -> bool>,
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        Provider {
            definition,
            eager_create,
            condition,
            constructor,
            clone_instance,
            eager_create_function,
            binding_providers: None,
            binding_definitions: None,
        }
    }

    pub(crate) fn never_construct(name: Cow<'static, str>, scope: Scope) -> Self {
        Provider {
            definition: Definition::new::<T>(name, scope, None, false),
            eager_create: false,
            condition: None,
            constructor: Constructor::None,
            clone_instance: None,
            eager_create_function: EagerCreateFunction::None,
            binding_providers: None,
            binding_definitions: None,
        }
    }
}

/// Represents a [`Provider`] that erased its type.
pub struct DynProvider {
    definition: Definition,
    eager_create: bool,
    condition: Option<fn(&Context) -> bool>,
    eager_create_function: EagerCreateFunction,
    binding_providers: Option<Vec<DynProvider>>,
    binding_definitions: Option<Vec<Definition>>,
    origin: Box<dyn Any>,
}

impl DynProvider {
    /// Returns the [`Definition`] of the provider.
    pub fn definition(&self) -> &Definition {
        &self.definition
    }

    /// Returns whether the provider is eager create.
    pub fn eager_create(&self) -> bool {
        self.eager_create
    }

    /// Returns definitions of the binding providers.
    pub fn binding_definitions(&self) -> Option<&Vec<Definition>> {
        self.binding_definitions.as_ref()
    }

    /// Returns a reference of the origin [`Provider`].
    pub fn as_provider<T: 'static>(&self) -> Option<&Provider<T>> {
        self.origin.downcast_ref::<Provider<T>>()
    }

    /// Returns an option of the condition function.
    pub fn condition(&self) -> Option<fn(&Context) -> bool> {
        self.condition
    }

    pub(crate) fn key(&self) -> &Key {
        &self.definition.key
    }

    pub(crate) fn eager_create_function(&self) -> EagerCreateFunction {
        self.eager_create_function.clone()
    }

    pub(crate) fn binding_providers(&mut self) -> Option<Vec<DynProvider>> {
        self.binding_providers.take()
    }
}

impl<T: 'static> From<Provider<T>> for DynProvider {
    fn from(mut value: Provider<T>) -> Self {
        Self {
            definition: value.definition.clone(),
            eager_create: value.eager_create,
            condition: value.condition,
            eager_create_function: value.eager_create_function.clone(),
            binding_providers: value.binding_providers.take(),
            binding_definitions: value.binding_definitions.clone(),
            origin: Box::new(value),
        }
    }
}

fn sync_constructor<T, U, F>(name: Cow<'static, str>, transform: F) -> Rc<dyn Fn(&mut Context) -> U>
where
    T: 'static,
    F: Fn(T) -> U + 'static,
{
    let constructor = move |cx: &mut Context| {
        let instance = cx.resolve_with_name(name.clone());
        transform(instance)
    };

    Rc::new(constructor)
}

fn sync_eager_create_function<T: 'static>() -> fn(&mut Context, Cow<'static, str>) {
    |cx, name| {
        cx.just_create::<T>(name);
    }
}

#[allow(clippy::type_complexity)]
fn async_constructor<T, U, F>(
    name: Cow<'static, str>,
    transform: F,
) -> Rc<dyn for<'a> Fn(&'a mut Context) -> BoxFuture<'a, U>>
where
    T: 'static,
    F: Fn(T) -> U + 'static + Clone,
{
    fn helper<'a, F, T, U>(
        cx: &'a mut Context,
        name: Cow<'static, str>,
        transform: F,
    ) -> BoxFuture<'a, U>
    where
        T: 'static,
        F: Fn(T) -> U + 'static,
    {
        async move {
            let instance = cx.resolve_with_name_async(name).await;
            transform(instance)
        }
        .boxed()
    }

    Rc::new(move |cx| helper(cx, name.clone(), transform.clone()))
}

fn async_eager_create_function<T: 'static>(
) -> for<'a> fn(&'a mut Context, Cow<'static, str>) -> BoxFuture<'a, ()> {
    |cx, name| {
        async {
            cx.just_create_async::<T>(name).await;
        }
        .boxed()
    }
}

macro_rules! define_provider_common {
    (
        $provider:ident,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        /// Represents a specialized [`Provider`].
        ///
        #[doc = concat!("Use the [`", stringify!($function), "`] function to create this provider.")]
        pub struct $provider<T> {
            constructor: Constructor<T>,
            name: Cow<'static, str>,
            eager_create: bool,
            condition: Option<fn(&Context) -> bool>,
            bind_closures: Vec<Box<dyn FnOnce(Definition, bool, Option<fn(&Context) -> bool>) -> DynProvider>>,
        }

        impl<T> $provider<T> {
            /// Sets the name of the provider.
            pub fn name<N>(mut self, name: N) -> Self
            where
                N: Into<Cow<'static, str>>,
            {
                self.name = name.into();
                self
            }

            /// Sets whether the provider is eager to create.
            pub fn eager_create(mut self, eager_create: bool) -> Self {
                self.eager_create = eager_create;
                self
            }

            /// Sets whether or not to insert the provider into the [`Context`] based on the condition.
            pub fn condition(mut self, condition: Option<fn(&Context) -> bool>) -> Self {
                self.condition = condition;
                self
            }
        }

        impl<T: 'static $(+ $bound)*> From<$provider<T>> for DynProvider {
            fn from(value: $provider<T>) -> Self {
                DynProvider::from(Provider::from(value))
            }
        }
    };
}

macro_rules! define_provider_sync {
    (
        $provider:ident,
        $scope:expr,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::{", stringify!($function), ", ", stringify!($provider), "};")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()));")]
        /// }
        /// ```
        pub fn $function<T, C>(constructor: C) -> $provider<T>
        where
            C: Fn(&mut Context) -> T + 'static,
        {
            $provider {
                constructor: Constructor::Sync(Rc::new(constructor)),
                name: Cow::Borrowed(""),
                eager_create: false,
                condition: None,
                bind_closures: Vec::new(),
            }
        }

        impl<T: 'static> $provider<T> {
            /// Create a provider of type [`Provider<U>`], save it to the current provider.
            ///
            /// This method accepts a parameter of `fn(T) -> U`, which in combination
            /// with the current provider's constructor of type `fn(&mut Context) -> T`,
            /// creates a `Provider<U>` with constructor `fn(&mut Context) -> U`
            /// and other fields consistent with the current provider.
            ///
            /// All bound providers will be registered together
            /// when the current provider is registered in the [`Context`].
            ///
            /// # Example
            ///
            /// ```rust
            /// use std::{fmt::Debug, rc::Rc, sync::Arc};
            ///
            #[doc = concat!("use rudi::{", stringify!($function), ", Provider, ", stringify!($provider), "};")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Rc<dyn Debug> {
            ///     Rc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: ", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()))")]
            ///         .bind(Rc::new)
            ///         .bind(Arc::new)
            ///         .bind(Box::new)
            ///         .bind(into_debug);
            ///
            ///     let p: Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static $(+ $bound)*,
                F: Fn(T) -> U + 'static,
            {
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&Context) -> bool>| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
                        condition,
                        Constructor::Sync(sync_constructor(name, transform)),
                        $clone_instance,
                        EagerCreateFunction::Sync(
                            sync_eager_create_function::<U>()
                        ),
                    )
                    .into()
                };

                let bind_closure = Box::new(bind_closure);
                self.bind_closures.push(bind_closure);

                self
            }
        }

        impl<T: 'static $(+ $bound)*> From<$provider<T>> for Provider<T> {
            fn from(value: $provider<T>) -> Self {
                let $provider {
                    constructor,
                    name,
                    eager_create,
                    condition,
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    $scope,
                    eager_create,
                    condition,
                    constructor,
                    $clone_instance,
                    EagerCreateFunction::Sync(
                        sync_eager_create_function::<T>()
                    ),
                );

                if bind_closures.is_empty() {
                    return provider;
                }

                let definition = &provider.definition;

                let (definitions, providers) = bind_closures.into_iter()
                    .map(|bind_closure| {
                        let provider = bind_closure(definition.clone(), eager_create, condition);
                        (provider.definition.clone(), provider)
                    })
                    .unzip();

                provider.binding_definitions = Some(definitions);
                provider.binding_providers = Some(providers);

                provider
            }
        }
    };
}

macro_rules! define_provider_async {
    (
        $provider:ident,
        $scope:expr,
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("Create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, ", stringify!($provider), "};")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: ", stringify!($provider), "<A> =")]
        #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed());")]
        /// }
        /// ```
        pub fn $function<T, C>(constructor: C) -> $provider<T>
        where
            C: for<'a> Fn(&'a mut Context) -> BoxFuture<'a, T> + 'static,
        {
            $provider {
                constructor: Constructor::Async(Rc::new(constructor)),
                name: Cow::Borrowed(""),
                eager_create: false,
                condition: None,
                bind_closures: Vec::new(),
            }
        }

        impl<T: 'static> $provider<T> {
            /// Create a provider of type [`Provider<U>`], save it to the current provider.
            ///
            /// This method accepts a parameter of `fn(T) -> U`, which in combination
            /// with the current provider's constructor of type `async fn(&mut Context) -> T`,
            /// creates a `Provider<U>` with constructor `async fn(&mut Context) -> U`
            /// and other fields consistent with the current provider.
            ///
            /// All bound providers will be registered together
            /// when the current provider is registered in the [`Context`].
            ///
            /// # Example
            ///
            /// ```rust
            /// use std::{fmt::Debug, rc::Rc, sync::Arc};
            ///
            #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt, Provider, ", stringify!($provider), "};")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Rc<dyn Debug> {
            ///     Rc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: ", stringify!($provider), "<A> =")]
            #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed())")]
            ///             .bind(Rc::new)
            ///             .bind(Arc::new)
            ///             .bind(Box::new)
            ///             .bind(into_debug);
            ///
            ///     let p: Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static $(+ $bound)*,
                F: Fn(T) -> U + 'static + Clone,
            {
                let bind_closure = |definition: Definition, eager_create: bool, condition: Option<fn(&Context) -> bool>| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
                        condition,
                        Constructor::Async(async_constructor(name, transform)),
                        $clone_instance,
                        EagerCreateFunction::Async(
                            async_eager_create_function::<U>()
                        ),
                    )
                    .into()
                };

                let bind_closure = Box::new(bind_closure);
                self.bind_closures.push(bind_closure);

                self
            }
        }

        impl<T: 'static $(+ $bound)*> From<$provider<T>> for Provider<T> {
            fn from(value: $provider<T>) -> Self {
                let $provider {
                    constructor,
                    name,
                    eager_create,
                    condition,
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    $scope,
                    eager_create,
                    condition,
                    constructor,
                    $clone_instance,
                    EagerCreateFunction::Async(
                        async_eager_create_function::<T>()
                    ),
                );

                if bind_closures.is_empty() {
                    return provider;
                }

                let definition = &provider.definition;

                let (definitions, providers) = bind_closures.into_iter()
                    .map(|bind_closure| {
                        let provider = bind_closure(definition.clone(), eager_create, condition);
                        (provider.definition.clone(), provider)
                    })
                    .unzip();

                provider.binding_definitions = Some(definitions);
                provider.binding_providers = Some(providers);

                provider
            }
        }
    };
}

define_provider_common!(SingletonProvider, singleton, Some(Clone::clone), + Clone);
define_provider_common!(TransientProvider, transient, None,);
define_provider_common!(SingleOwnerProvider, single_owner, None,);
define_provider_common!(SingletonAsyncProvider, singleton_async, Some(Clone::clone), + Clone);
define_provider_common!(TransientAsyncProvider, transient_async, None,);
define_provider_common!(SingleOwnerAsyncProvider, single_owner_async, None,);

define_provider_sync!(SingletonProvider, Scope::Singleton, singleton, Some(Clone::clone), + Clone);
define_provider_sync!(TransientProvider, Scope::Transient, transient, None,);
define_provider_sync!(SingleOwnerProvider, Scope::SingleOwner, single_owner, None,);

define_provider_async!(SingletonAsyncProvider, Scope::Singleton, singleton_async, Some(Clone::clone), + Clone);
define_provider_async!(
    TransientAsyncProvider,
    Scope::Transient,
    transient_async,
    None,
);
define_provider_async!(
    SingleOwnerAsyncProvider,
    Scope::SingleOwner,
    single_owner_async,
    None,
);
