use std::{
    cell::Cell,
    collections::HashMap,
    fmt,
    ops::Deref,
    sync::{Arc, Weak},
};

pub trait Observable {
    type Item;

    fn subscribe(
        &self,
        callback: Box<dyn FnMut(&Self::Item)>,
    ) -> Option<Unsub<Self::Item>>;

    fn with<F, O>(&self, f: F) -> O
    where
        Self: Sized,
        F: FnOnce(Option<&Self::Item>) -> O;
}

impl<T> Observable for Box<T>
where
    T: Observable,
{
    type Item = T::Item;

    fn subscribe(
        &self,
        callback: Box<dyn FnMut(&Self::Item)>,
    ) -> Option<Unsub<Self::Item>> {
        self.deref().subscribe(callback)
    }

    fn with<F, O>(&self, f: F) -> O
    where
        Self: Sized,
        F: FnOnce(Option<&Self::Item>) -> O,
    {
        self.deref().with(f)
    }
}

#[derive(Educe)]
#[educe(Clone, Default(bound))]
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
        self.0.observers.set(Some(observers));
    }

    pub fn new(value: T) -> Self {
        Self::from(value)
    }

    pub fn set(&mut self, value: T) {
        self.0.value.set(Some(value));

        self.notify();
    }

    pub fn set_with<U>(&mut self, f: impl FnOnce(&mut T) -> U) -> U {
        let mut value = self.0.value.take().unwrap();

        let ret = f(&mut value);

        self.0.value.set(Some(value));

        self.notify();

        ret
    }

    pub fn observer(&self) -> Observer<T> {
        let id = self.0.next_id.update(|c| c + 1);

        Observer {
            source: Arc::downgrade(&self.0),
            id,
        }
    }
}

#[derive(Educe)]
#[educe(Default(bound))]
struct SourceInner<T> {
    #[educe(Default(expression = "Cell::new(Some(T::default()))"))]
    value: Cell<Option<T>>,
    #[educe(Default(expression = "Cell::new(Some(Default::default()))"))]
    #[allow(clippy::type_complexity)]
    observers: Cell<Option<HashMap<usize, Box<dyn FnMut(&T)>>>>,
    #[educe(Default(expression = "Cell::new(0)"))]
    next_id: Cell<usize>,
}

assert_impl_all!(SourceInner<()>: Send, Sync);

unsafe impl<T> Send for SourceInner<T> where T: Send {}
unsafe impl<T> Sync for SourceInner<T> where T: Send {}

impl<T> From<T> for SourceInner<T> {
    fn from(value: T) -> Self {
        Self {
            value: Cell::new(Some(value)),
            observers: Cell::new(Some(HashMap::default())),
            next_id: Cell::new(0),
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

/// Formats "N/A" if the source has dropped.
impl<T> fmt::Debug for Observer<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|v| {
            if let Some(v) = v {
                v.fmt(f)
            } else {
                f.write_str("N/A")
            }
        })
    }
}

/// Formats "N/A" if the source has dropped.
impl<T> fmt::Display for Observer<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with(|v| {
            if let Some(v) = v {
                v.fmt(f)
            } else {
                f.write_str("N/A")
            }
        })
    }
}

impl<T> Observable for Observer<T> {
    type Item = T;

    fn subscribe(
        &self,
        mut callback: Box<dyn FnMut(&Self::Item)>,
    ) -> Option<Unsub<Self::Item>> {
        if let Some(source) = self.source.upgrade() {
            let v = source.value.take().unwrap();
            let mut observers = source.observers.take().unwrap();

            // First of all, we need to pass the current value to
            // the callback
            callback(&v);

            // After this, we just need to save the callback
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

    fn with<F, O>(&self, f: F) -> O
    where
        Self: Sized,
        F: FnOnce(Option<&Self::Item>) -> O,
    {
        if let Some(source) = self.source.upgrade() {
            // Borrow v
            let v = source.value.take();

            let o = f(v.as_ref());

            // Return v
            source.value.set(v);

            o
        } else {
            f(None)
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
