use std::{
    any::Any,
    collections::{
        hash_map::{Iter, Keys},
        HashMap,
    },
};

use crate::{DynProvider, Key, Provider};

pub(crate) struct SingletonInstance<T> {
    instance: T,
    clone: fn(&T) -> T,
}

impl<T> SingletonInstance<T> {
    pub(crate) fn new(instance: &T, clone: fn(&T) -> T) -> Self {
        Self {
            instance: clone(instance),
            clone,
        }
    }

    fn clone_instance(&self) -> T {
        (self.clone)(&self.instance)
    }
}

#[derive(Default)]
pub(crate) struct SingletonRegistry {
    registry: HashMap<Key, Box<dyn Any>>,
}

impl SingletonRegistry {
    #[track_caller]
    pub(crate) fn insert<T: 'static>(&mut self, key: Key, instance: SingletonInstance<T>) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a singleton with the same key into the context,
        // the provider must be inserted first, followed by the singleton,
        // and the checking of `allow_override` has already been done when the provider is inserted.

        self.registry.insert(key, Box::new(instance));
    }

    pub(crate) fn get<T: 'static>(&self, key: &Key) -> Option<T> {
        Some(
            self.registry
                .get(key)?
                .downcast_ref::<SingletonInstance<T>>()?
                .clone_instance(),
        )
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<Box<dyn Any>> {
        self.registry.remove(key)
    }

    pub(crate) fn len(&self) -> usize {
        self.registry.len()
    }
}

#[derive(Default)]
pub(crate) struct ProviderRegistry {
    registry: HashMap<Key, DynProvider>,
}

impl ProviderRegistry {
    #[track_caller]
    pub(crate) fn insert(&mut self, provider: DynProvider, allow_override: bool) {
        let definition = provider.definition();
        let key = provider.key().clone();

        if !self.registry.contains_key(&key) {
            #[cfg(feature = "debug-print")]
            tracing::debug!("(+) insert new: {:?}", definition);
        } else if allow_override {
            #[cfg(feature = "debug-print")]
            tracing::warn!("(!) override by `key`: {:?}", definition);
        } else {
            panic!(
                "already existing a provider with the same `key`: {:?}",
                definition
            );
        }

        self.registry.insert(key, provider);
    }

    pub(crate) fn get<T: 'static>(&self, key: &Key) -> Option<&Provider<T>> {
        self.registry.get(key)?.as_provider()
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynProvider> {
        self.registry.remove(key)
    }

    pub(crate) fn len(&self) -> usize {
        self.registry.len()
    }

    pub(crate) fn keys(&self) -> Keys<'_, Key, DynProvider> {
        self.registry.keys()
    }

    pub(crate) fn iter(&self) -> Iter<'_, Key, DynProvider> {
        self.registry.iter()
    }
}
