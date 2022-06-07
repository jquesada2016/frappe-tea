use std::{
    cell::Cell,
    collections::HashMap,
    fmt,
    sync::{Arc, Weak},
};

pub trait Observable {
    type Item;

    fn subscribe(
        &self,
        callback: Box<dyn FnMut(&Self::Item)>,
    ) -> Option<Unsub<Self::Item>>;
}

#[derive(Default, Educe)]
#[educe(Clone)]
pub struct Source<T>(Arc<SourceInner<T>>);

assert_impl_all!(Source<()>: Send, Sync);

impl<T> fmt::Debug for Source<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.0.value.take().unwrap();

        f.debug_tuple("Source").field(&v).finish()?;

        self.0.value.set(Some(v));

        Ok(())
    }
}

impl<T> From<T> for Source<T> {
    fn from(data: T) -> Self {
        Self(Arc::new(SourceInner::from(data)))
    }
}

impl<T> Source<T> {
    fn notify(&self) {
        // Grab what we need
        let v = self.0.value.take().unwrap();
        let mut observers = self.0.observers.take().unwrap();

        // Pass the data to all observers
        observers.values_mut().for_each(|o| o(&v));

        // Give the value back before anyone notices
        self.0.value.set(Some(v));
    }

    pub fn set(&self, value: T) {
        self.0.value.set(Some(value));

        self.notify();
    }

    pub fn set_with<U>(&self, f: impl FnOnce(&mut T) -> U) -> U {
        let mut value = self.0.value.take().unwrap();

        let ret = f(&mut value);

        self.0.value.set(Some(value));

        self.notify();

        ret
    }
}

#[derive(Default)]
struct SourceInner<T> {
    value: Cell<Option<T>>,
    #[allow(clippy::type_complexity)]
    observers: Cell<Option<HashMap<usize, Box<dyn FnMut(&T)>>>>,
    next_id: Cell<usize>,
}

assert_impl_all!(SourceInner<()>: Send, Sync);

unsafe impl<T> Send for SourceInner<T> where T: Send {}
unsafe impl<T> Sync for SourceInner<T> where T: Send {}

impl<T> From<T> for SourceInner<T> {
    fn from(value: T) -> Self {
        Self {
            value: Cell::new(Some(value)),
            observers: Cell::default(),
            next_id: Cell::default(),
        }
    }
}

pub struct Observer<T> {
    source: Weak<SourceInner<T>>,
    id: usize,
}

assert_impl_all!(Observer<()>: Send, Sync);

impl<T> Clone for Observer<T> {
    fn clone(&self) -> Self {
        let id = if let Some(source) = self.source.upgrade() {
            source.next_id.update(|c| c + 1)
        } else {
            0
        };

        Self {
            source: self.source.clone(),
            id,
        }
    }
}

impl<T> Observable for Observer<T> {
    type Item = T;

    fn subscribe(
        &self,
        mut callback: Box<dyn FnMut(&Self::Item)>,
    ) -> Option<Unsub<Self::Item>> {
        if let Some(source) = self.source.upgrade() {
            // First of all, we need to pass the current value to
            // the callback
            let v = source.value.take().unwrap();

            callback(&v);

            // After this, we just need to save the callback
            let mut observers = source.observers.take().unwrap();
            observers.insert(self.id, callback);

            // Give everything we borrowed back
            source.observers.set(Some(observers));
            source.value.set(Some(v));

            Some(Unsub {
                id: self.id,
                source: self.source.clone(),
            })
        }
        // If `Source` has already dropped, then notify the subscriber
        else {
            None
        }
    }
}

pub struct Unsub<T> {
    source: Weak<SourceInner<T>>,
    id: usize,
}

impl<T> Unsub<T> {
    pub fn unsubscribe(self) {
        if let Some(source) = self.source.upgrade() {
            // Take what we need
            let mut observers = source.observers.take().unwrap();

            // Remove the observer
            observers.remove(&self.id);

            // Give back what we took
            source.observers.set(Some(observers));
        }
    }
}
