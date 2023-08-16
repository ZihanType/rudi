use std::{any::Any, borrow::Cow, rc::Rc};

use crate::{
    context::Context,
    definition::{Color, Definition, Scope},
    BoxFuture, FutureExt, Key,
};

/// A trait for giving a type a default [`Provider`].
///
/// Define this trait so that the purpose is not to be implemented manually,
/// but to use the `#[Singleton]` or `#[Transient]` attribute macros to generate the implementation.
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
}

impl<T> Clone for Constructor<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Async(c) => Self::Async(Rc::clone(c)),
            Self::Sync(c) => Self::Sync(Rc::clone(c)),
        }
    }
}

#[derive(Clone)]
pub(crate) enum EagerCreateFunction {
    Async(for<'a> fn(&'a mut Context, Cow<'static, str>) -> BoxFuture<'a, ()>),
    Sync(fn(&mut Context, Cow<'static, str>)),
}

/// Represents the provider of an instance of type `T`.
///
/// This struct is just a generic, intermediate representation of `Provider`,
/// there is no pub method to direct create this struct,
/// Please use the following functions or attribute macros to create the various `Provider` types that implement `Into<Provider>`:
/// - functions
///   - [`singleton`](crate::singleton)
///   - [`transient`](crate::transient)
///   - [`singleton_async`](crate::singleton_async)
///   - [`transient_async`](crate::transient_async)
/// - attribute macros
///   - [`Singleton`](crate::Singleton)
///   - [`Transient`](crate::Transient)
pub struct Provider<T> {
    definition: Definition,
    eager_create: bool,
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
        eager_create: bool,
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        let definition = Definition::new::<T>(
            name,
            match clone_instance {
                Some(_) => Scope::Singleton,
                None => Scope::Transient,
            },
            match constructor {
                Constructor::Async(_) => Color::Async,
                Constructor::Sync(_) => Color::Sync,
            },
        );

        Provider {
            definition,
            eager_create,
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
        constructor: Constructor<T>,
        clone_instance: Option<fn(&T) -> T>,
        eager_create_function: EagerCreateFunction,
    ) -> Self {
        Provider {
            definition,
            eager_create,
            constructor,
            clone_instance,
            eager_create_function,
            binding_providers: None,
            binding_definitions: None,
        }
    }
}

impl<T: 'static + Clone> Provider<T> {
    pub(crate) fn standalone(name: Cow<'static, str>, instance: T) -> Self {
        Provider {
            definition: Definition::new::<T>(name, Scope::Singleton, Color::Sync),
            eager_create: false,
            constructor: Constructor::Sync(Rc::new(move |_| instance.clone())),
            clone_instance: Some(Clone::clone),
            eager_create_function: EagerCreateFunction::Sync(sync_eager_create_function::<T>()),
            binding_providers: None,
            binding_definitions: None,
        }
    }
}

/// Represents a [`Provider`] that erases a generic type.
pub struct DynProvider {
    definition: Definition,
    eager_create: bool,
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

    /// Returns the reference of the origin [`Provider`].
    pub fn as_provider<T: 'static>(&self) -> Option<&Provider<T>> {
        self.origin.downcast_ref::<Provider<T>>()
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
        cx.resolve_with_name::<T>(name);
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
    #[allow(clippy::needless_pass_by_ref_mut)] // false positive
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
            cx.resolve_with_name_async::<T>(name).await;
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
            bind_closures: Vec<Box<dyn FnOnce(Definition, bool) -> DynProvider>>,
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

            /// Sets whether the provider is eager create.
            pub fn eager_create(mut self, eager_create: bool) -> Self {
                self.eager_create = eager_create;
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
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::", stringify!($function), ";")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: rudi::", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()));")]
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
            #[doc = concat!("use rudi::", stringify!($function), ";")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Rc<dyn Debug> {
            ///     Rc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: rudi::", stringify!($provider), "<A> = ", stringify!($function), "(|cx| A(cx.resolve()))")]
            ///         .bind(Rc::new)
            ///         .bind(Arc::new)
            ///         .bind(Box::new)
            ///         .bind(into_debug);
            ///
            ///     let p: rudi::Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static $(+ $bound)*,
                F: Fn(T) -> U + 'static,
            {
                let bind_closure = |definition: Definition, eager_create: bool| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
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
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    eager_create,
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
                        let provider = bind_closure(definition.clone(), eager_create);
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
        $function:ident,
        $clone_instance:expr,
        $(+ $bound:ident)*
    ) => {
        #[doc = concat!("Create a [`", stringify!($provider), "`] instance")]
        ///
        /// # Example
        ///
        /// ```rust
        #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt};")]
        ///
        /// #[derive(Clone)]
        /// struct A(i32);
        ///
        /// fn main() {
        #[doc = concat!("    let _: rudi::", stringify!($provider), "<A> =")]
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
            #[doc = concat!("use rudi::{", stringify!($function), ", FutureExt};")]
            ///
            /// #[derive(Clone, Debug)]
            /// struct A(i32);
            ///
            /// fn into_debug(a: A) -> Rc<dyn Debug> {
            ///     Rc::new(a)
            /// }
            ///
            /// fn main() {
            #[doc = concat!("    let p: rudi::", stringify!($provider), "<A> =")]
            #[doc = concat!("        ", stringify!($function), "(|cx| async { A(cx.resolve_async().await) }.boxed())")]
            ///             .bind(Rc::new)
            ///             .bind(Arc::new)
            ///             .bind(Box::new)
            ///             .bind(into_debug);
            ///
            ///     let p: rudi::Provider<A> = p.into();
            ///
            ///     assert_eq!(p.binding_definitions().unwrap().len(), 4);
            /// }
            /// ```
            pub fn bind<U, F>(mut self, transform: F) -> Self
            where
                U: 'static $(+ $bound)*,
                F: Fn(T) -> U + 'static + Clone,
            {
                let bind_closure = |definition: Definition, eager_create: bool| {
                    let name = definition.key.name.clone();

                    Provider::with_definition(
                        definition.bind::<U>(),
                        eager_create,
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
                    bind_closures,
                } = value;

                let mut provider = Provider::with_name(
                    name,
                    eager_create,
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
                        let provider = bind_closure(definition.clone(), eager_create);
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
define_provider_common!(SingletonAsyncProvider, singleton_async, Some(Clone::clone), + Clone);
define_provider_common!(TransientAsyncProvider, transient_async, None,);

define_provider_sync!(SingletonProvider, singleton, Some(Clone::clone), + Clone);
define_provider_sync!(TransientProvider, transient, None,);

define_provider_async!(SingletonAsyncProvider, singleton_async, Some(Clone::clone), + Clone);
define_provider_async!(TransientAsyncProvider, transient_async, None,);
