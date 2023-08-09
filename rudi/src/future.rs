use std::{future::Future, pin::Pin};

/// An owned dynamically typed [`Future`] for use in cases where you can't
/// statically type your result or need to add some indirection.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

impl<T: ?Sized> FutureExt for T where T: Future {}

/// An extension trait for `Future`s that provides a variety of convenient
/// adapters.
pub trait FutureExt: Future {
    /// Wrap the future in a Box, pinning it.
    ///
    /// This method is only available when the `std` or `alloc` feature of this
    /// library is activated, and it is activated by default.
    fn boxed<'a>(self) -> BoxFuture<'a, Self::Output>
    where
        Self: Sized + 'a,
    {
        Box::pin(self)
    }
}
