use std::{
    borrow::Cow,
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use crate::{Color, Scope, Type};

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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.ty.cmp(&other.ty) {
            Ordering::Equal => {}
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
    /// - [`SingleOwnerProvider::bind`](crate::SingleOwnerProvider::bind)
    /// - [`SingletonAsyncProvider::bind`](crate::SingletonAsyncProvider::bind)
    /// - [`TransientAsyncProvider::bind`](crate::TransientAsyncProvider::bind)
    /// - [`SingleOwnerAsyncProvider::bind`](crate::SingleOwnerAsyncProvider::bind)
    pub origin: Option<Type>,
    /// The scope of the provider.
    pub scope: Scope,
    /// The color of the constructor.
    pub color: Option<Color>,
    /// Whether the provider is conditional.
    pub conditional: bool,
}

impl Definition {
    pub(crate) fn new<T: 'static>(
        name: Cow<'static, str>,
        scope: Scope,
        color: Option<Color>,
        conditional: bool,
    ) -> Self {
        Self {
            key: Key::new::<T>(name),
            origin: None,
            scope,
            color,
            conditional,
        }
    }

    pub(crate) fn bind<T: 'static>(self) -> Definition {
        let Definition {
            key: Key { name, ty },
            scope,
            color,
            conditional,
            origin: _origin,
        } = self;

        Self {
            key: Key::new::<T>(name),
            origin: Some(ty),
            scope,
            color,
            conditional,
        }
    }
}
