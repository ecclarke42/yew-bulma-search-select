use std::sync::Arc;

// Use the Box to make sure we're not doing a Arc::ptr_eq on dyn objects (since rust doesn't like that)
type SelectFilterContainer<T> = Box<dyn Fn(&T, &str) -> bool>;

pub struct SelectFilter<T> {
    inner: Arc<SelectFilterContainer<T>>,
}

impl<T> PartialEq for SelectFilter<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> SelectFilter<T> {
    pub fn new<F: Fn(&T, &str) -> bool + 'static>(f: F) -> Self {
        Self {
            inner: Arc::new(Box::new(f) as SelectFilterContainer<T>),
        }
    }

    // TODO: impl Fn when traits stabilize?
    pub fn call(&self, item: &T, input: &str) -> bool {
        (self.inner)(item, input)
    }
}

impl<T, F: Fn(&T, &str) -> bool + 'static> From<F> for SelectFilter<T> {
    fn from(f: F) -> Self {
        SelectFilter::new(f)
    }
}

impl<T> Clone for SelectFilter<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

type SelectDisplayContainer<T> = Box<dyn Fn(&T) -> String>;

pub struct SelectDisplay<T> {
    inner: Arc<SelectDisplayContainer<T>>,
}

impl<T> PartialEq for SelectDisplay<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl<T> SelectDisplay<T> {
    pub fn new<F: Fn(&T) -> String + 'static>(f: F) -> Self {
        Self {
            inner: Arc::new(Box::new(f) as SelectDisplayContainer<T>),
        }
    }

    // TODO: impl Fn when traits stabilize?
    pub fn call(&self, item: &T) -> String {
        (self.inner)(item)
    }
}

impl<T, F: Fn(&T) -> String + 'static> From<F> for SelectDisplay<T> {
    fn from(f: F) -> Self {
        SelectDisplay::new(f)
    }
}

impl<T> Clone for SelectDisplay<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
