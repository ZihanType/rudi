/// Represents the scope of the instance
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    /// singleton scope.
    ///
    /// 1. the constructor run only once.
    /// 2. the type implements [`Clone`] trait.
    /// 3. instances taken from the container are owned.
    Singleton,
    /// transient scope.
    ///
    /// 1. the constructor run every time.
    /// 2. instances taken from the container are owned.
    Transient,
}

/// Represents the color of the function, i.e., async or sync.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Color {
    /// async function
    Async,
    /// sync function
    Sync,
}
