use std::any::Any;

/// Represents a singleton instance.
pub struct SingletonInstance<T> {
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

    /// Returns the owned instance.
    pub fn get_owned(&self) -> T {
        (self.clone)(&self.instance)
    }

    /// Returns a reference to the instance.
    pub fn get_ref(&self) -> &T {
        &self.instance
    }
}

/// Represents a [`SingletonInstance`] that erased its type.
pub struct DynSingletonInstance {
    origin: Box<dyn Any>,
}

impl DynSingletonInstance {
    /// Returns the reference of the origin [`SingletonInstance`].
    pub fn as_singleton<T: 'static>(&self) -> Option<&SingletonInstance<T>> {
        self.origin.downcast_ref::<SingletonInstance<T>>()
    }
}

impl<T: 'static> From<SingletonInstance<T>> for DynSingletonInstance {
    fn from(value: SingletonInstance<T>) -> Self {
        Self {
            origin: Box::new(value),
        }
    }
}
