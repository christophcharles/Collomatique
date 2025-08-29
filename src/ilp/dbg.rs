use std::sync::Arc;

pub struct Debuggable<T: ?Sized> {
    obj: Arc<T>,
    debug_payload: &'static str,
}

impl<T: ?Sized> Debuggable<T> {
    pub fn new(obj: Arc<T>, debug_payload: &'static str) -> Debuggable<T> {
        Debuggable { obj, debug_payload }
    }
}

#[macro_export]
macro_rules! debuggable {
    ($($body:tt)+) => {
        $crate::ilp::dbg::Debuggable::new(
            std::sync::Arc::new($($body)+),
            stringify!($($body)+)
        )
    };
}

impl<T: ?Sized> Clone for Debuggable<T> {
    fn clone(&self) -> Self {
        Debuggable {
            obj: self.obj.clone(),
            debug_payload: self.debug_payload,
        }
    }
}

impl<T: ?Sized> std::fmt::Debug for Debuggable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.debug_payload)
    }
}

impl<T: ?Sized> std::ops::Deref for Debuggable<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.obj
    }
}
