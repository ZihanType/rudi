use std::{
    any::{self, TypeId},
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// Represents a type.
#[derive(Clone, Copy, Debug)]
pub struct Type {
    /// The name of the type.
    pub name: &'static str,
    /// The unique identifier of the type.
    pub id: TypeId,
}

impl Type {
    pub(crate) fn new<T: 'static>() -> Type {
        Type {
            name: any::type_name::<T>(),
            id: TypeId::of::<T>(),
        }
    }
}

impl PartialEq for Type {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Type {}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Type {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for Type {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
