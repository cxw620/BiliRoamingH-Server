use std::{borrow::Cow, collections::HashMap};

pub trait ContextT<T: ContextInner> {
    /// Consumes ctx and returns inner params prepared
    fn into_inner(self) -> T;
    /// Returns proxy that should be used when requesting upstream
    fn proxy(&self) -> Option<&'static str> {
        None
    }
}

/// General wrapper for context between [`service`] and backend layer
///
/// Including:
/// - Request params
/// - Proxy info
/// - ...
pub struct Context<T: ContextInner> {
    inner: T,
    proxy: Option<&'static str>,
}

impl<T: ContextInner> Context<T> {
    pub fn new(inner: T, proxy: Option<&'static str>) -> Context<T> {
        Self { inner, proxy }
    }
}

impl<T: ContextInner> ContextT<T> for Context<T> {
    fn into_inner(self) -> T {
        self.inner
    }

    fn proxy(&self) -> Option<&'static str> {
        self.proxy
    }
}

pub trait ContextInner: Sized + Send {}
