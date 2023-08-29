use std::collections::HashMap;

use crate::{DynProvider, DynSingletonInstance, Key, Provider, SingletonInstance};

#[derive(Default)]
pub(crate) struct SingletonRegistry {
    registry: HashMap<Key, DynSingletonInstance>,
}

impl SingletonRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynSingletonInstance> {
        &self.registry
    }

    pub(crate) fn insert<T: 'static>(&mut self, key: Key, instance: SingletonInstance<T>) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a singleton with the same key into the context,
        // the provider must be inserted first, followed by the singleton,
        // and the checking of `allow_override` has already been done when the provider is inserted.
        self.registry.insert(key, instance.into());
    }

    pub(crate) fn get_owned<T: 'static>(&self, key: &Key) -> Option<T> {
        Some(self.registry.get(key)?.as_singleton::<T>()?.get_owned())
    }

    pub(crate) fn get_ref<T: 'static>(&self, key: &Key) -> Option<&T> {
        Some(self.registry.get(key)?.as_singleton::<T>()?.get_ref())
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynSingletonInstance> {
        self.registry.remove(key)
    }
}

#[derive(Default)]
pub(crate) struct ProviderRegistry {
    registry: HashMap<Key, DynProvider>,
}

impl ProviderRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynProvider> {
        &self.registry
    }

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
}
