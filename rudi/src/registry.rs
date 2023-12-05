use std::collections::HashMap;

use crate::{DynProvider, DynSingle, Key, Provider};

#[derive(Default)]
pub(crate) struct SingleRegistry {
    registry: HashMap<Key, DynSingle>,
}

impl SingleRegistry {
    pub(crate) fn inner(&self) -> &HashMap<Key, DynSingle> {
        &self.registry
    }

    pub(crate) fn insert(&mut self, key: Key, single: DynSingle) {
        // There is no need to check the value of `allow_override` here,
        // because when inserting a provider and a single with the same key into the context,
        // the provider must be inserted first, followed by the single,
        // and the checking of `allow_override` has already been done when the provider is inserted.
        self.registry.insert(key, single);
    }

    pub(crate) fn get_owned<T: 'static>(&self, key: &Key) -> Option<T> {
        self.registry.get(key)?.as_single::<T>()?.get_owned()
    }

    pub(crate) fn get_ref<T: 'static>(&self, key: &Key) -> Option<&T> {
        Some(self.registry.get(key)?.as_single::<T>()?.get_ref())
    }

    pub(crate) fn contains(&self, key: &Key) -> bool {
        self.registry.contains_key(key)
    }

    pub(crate) fn remove(&mut self, key: &Key) -> Option<DynSingle> {
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
            #[cfg(feature = "tracing")]
            tracing::debug!("(+) insert new: {:?}", definition);
        } else if allow_override {
            #[cfg(feature = "tracing")]
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
