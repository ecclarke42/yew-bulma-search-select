use std::sync::Arc;

type SelectFilterContainer<T> = Arc<dyn Fn(&T, &str) -> bool>;

pub struct SelectFilter<T> {
    inner: SelectFilterContainer<T>,
}

impl<T> SelectFilter<T> {
    pub fn new<F: Fn(&T, &str) -> bool + 'static>(f: F) -> Self {
        Self {
            inner: Arc::new(f) as SelectFilterContainer<T>,
        }
    }

    // TODO: impl Fn when traits stabilize?
    pub fn call(&self, item: &T, input: &str) -> bool {
        (self.inner)(item, input)
    }
}

impl<T> Clone for SelectFilter<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

type SelectDisplayContainer<T> = Arc<dyn Fn(&T) -> String>;

pub struct SelectDisplay<T> {
    inner: SelectDisplayContainer<T>,
}

impl<T> SelectDisplay<T> {
    pub fn new<F: Fn(&T) -> String + 'static>(f: F) -> Self {
        Self {
            inner: Arc::new(f) as SelectDisplayContainer<T>,
        }
    }

    // TODO: impl Fn when traits stabilize?
    pub fn call(&self, item: &T) -> String {
        (self.inner)(item)
    }
}

impl<T> Clone for SelectDisplay<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
