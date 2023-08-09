use std::{
    any::{self, TypeId},
    borrow::Cow,
    hash::{Hash, Hasher},
};

/// Represents a type.
#[derive(Clone, Debug, Eq)]
pub struct Type {
    /// The name of the type.
    pub name: &'static str,
    /// The unique identifier of the type.
    pub id: TypeId,
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Type {
    pub(crate) fn new<T: 'static>() -> Type {
        Type {
            name: any::type_name::<T>(),
            id: TypeId::of::<T>(),
        }
    }
}

/// Represents a unique key for a provider.
#[derive(Clone, Debug, Eq)]
pub struct Key {
    /// The name of the provider.
    pub name: Cow<'static, str>,
    /// The type of the provider generic.
    pub ty: Type,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.name == other.name
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.name.hash(state);
    }
}

impl Key {
    pub(crate) fn new<T: 'static>(name: Cow<'static, str>) -> Self {
        Self {
            name,
            ty: Type::new::<T>(),
        }
    }
}

/// Represents how the constructor is run
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// singleton, constructor will be run only once.
    Singleton,
    /// transient, constructor will be run every time.
    Transient,
}

/// Represents the color of the constructor, i.e., async or sync.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    /// async, constructor must run in an async context
    Async,
    /// sync, constructor can run in both sync and async context
    Sync,
}

/// Represents a definition of a provider.
#[derive(Clone, Debug)]
pub struct Definition {
    /// The unique key of the provider.
    pub key: Key,
    /// The origin type of the provider.
    ///
    /// When the following methods are called, current definition represents the
    /// return type of the method, and this field represents the parameter type of the method:
    /// - [`SingletonProvider::bind`](crate::SingletonProvider::bind)
    /// - [`TransientProvider::bind`](crate::TransientProvider::bind)
    /// - [`SingletonAsyncProvider::bind`](crate::SingletonAsyncProvider::bind)
    /// - [`TransientAsyncProvider::bind`](crate::TransientAsyncProvider::bind)
    pub origin: Option<Type>,
    /// The scope of the provider.
    pub scope: Scope,
    /// The color of the provider.
    pub color: Color,
}

impl Definition {
    pub(crate) fn new<T: 'static>(name: Cow<'static, str>, scope: Scope, color: Color) -> Self {
        Self {
            key: Key::new::<T>(name),
            origin: None,
            scope,
            color,
        }
    }

    pub(crate) fn bind<T: 'static>(self) -> Definition {
        let Definition {
            key: Key { name, ty },
            scope,
            color,
            ..
        } = self;

        Self {
            key: Key::new::<T>(name),
            origin: Some(ty),
            scope,
            color,
        }
    }
}
