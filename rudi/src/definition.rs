use std::{
    borrow::Cow,
    hash::{Hash, Hasher},
};

use crate::Type;

/// Represents a unique key for a provider.
#[derive(Clone, Debug)]
pub struct Key {
    /// The name of the provider.
    pub name: Cow<'static, str>,
    /// The type of the provider generic.
    pub ty: Type,
}

impl Key {
    pub(crate) fn new<T: 'static>(name: Cow<'static, str>) -> Self {
        Self {
            name,
            ty: Type::new::<T>(),
        }
    }
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        self.ty == other.ty && self.name == other.name
    }
}

impl Eq for Key {}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.ty.cmp(&other.ty) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl Hash for Key {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ty.hash(state);
        self.name.hash(state);
    }
}

/// Represents how the constructor is run
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// singleton, constructor will be run only once.
    Singleton,
    /// transient, constructor will be run every time.
    Transient,
}

/// Represents the color of the constructor, i.e., async or sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    /// async, constructor must run in an async context
    Async,
    /// sync, constructor can run in both sync and async context
    Sync,
}

/// Represents a definition of a provider.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
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
