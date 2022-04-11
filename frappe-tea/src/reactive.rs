use std::{
    cell::{RefCell, UnsafeCell},
    fmt, ops,
    rc::{Rc, Weak},
};

// =============================================================================
//                              Traits
// =============================================================================

pub trait Observable {
    type Item;

    fn subscribe(
        &self,
        f: Box<dyn FnMut(Self::Item)>,
    ) -> Option<Box<dyn Unsubscribe>>;

    fn with(&self, f: Box<dyn FnOnce(&Self::Item)>)
    where
        Self::Item: 'static,
    {
        let mut f = Some(f);

        // We just need to sub/unsub
        let unsub = self.subscribe(Box::new(move |v| {
            let f = f.take().unwrap();

            f(&v);
        }));

        unsub.unwrap().unsubscribe();
    }

    fn map<F, B>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> B + 'static,
    {
        Map {
            observer: self,
            mapping_fn: Rc::new(RefCell::new(f)),
        }
    }
}

assert_obj_safe!(Observable<Item = ()>);

pub trait Unsubscribe {
    fn unsubscribe(self: Box<Self>);
}

assert_obj_safe!(Unsubscribe);

impl<T, I> Observable for Box<T>
where
    T: Observable<Item = I>,
{
    type Item = I;

    fn subscribe(
        &self,
        f: Box<dyn FnMut(Self::Item)>,
    ) -> Option<Box<dyn Unsubscribe>> {
        (**self).subscribe(f)
    }
}

// =============================================================================
//                         Structs and Impls
// =============================================================================

pub struct Map<O, F> {
    observer: O,
    mapping_fn: Rc<RefCell<F>>,
}

impl<O, F, B> Observable for Map<O, F>
where
    O: Observable,
    F: FnMut(O::Item) -> B + 'static,
    B: 'static,
{
    type Item = B;

    fn subscribe(
        &self,
        mut f: Box<dyn FnMut(Self::Item)>,
    ) -> Option<Box<dyn Unsubscribe>> {
        let mapping_fn = self.mapping_fn.clone();

        self.observer.subscribe(Box::new(move |v| {
            let v = mapping_fn.borrow_mut()(v);
            f(v);
        }))
    }
}

#[derive(Clone)]
pub struct Observer<T>(Weak<UnsafeCell<SharedState<T>>>);

impl<T> Observable for Observer<T>
where
    T: 'static,
{
    type Item = Ref<T>;

    fn subscribe(
        &self,
        mut f: Box<dyn FnMut(Self::Item)>,
    ) -> Option<Box<dyn Unsubscribe>> {
        if let Some(shared_state) = self.0.upgrade() {
            // Call `f` immediately so that it can get the current value
            let v_ref = Ref(self.0.clone());

            f(v_ref);

            let callbacks = unsafe { &mut (*shared_state.get()).callbacks };

            let index = callbacks.len();

            callbacks.push(Some(f));

            Some(Box::new(Unsub(index, self.0.clone())))
        } else {
            None
        }
    }
}

pub struct Ref<T>(Weak<UnsafeCell<SharedState<T>>>);

assert_not_impl_any!(Ref<()>: Clone);

impl<T> ops::Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // This is safe as long as `Ref<T>` does not impl `Clone`.
        // We also guarantee that this is the only immutable borrow of T,
        // since callbacks are executed synchronously and in order directly
        // after `Source<T>` is mutated
        unsafe { &(*self.0.upgrade().unwrap().get()).value }
    }
}

struct SharedState<T> {
    value: T,
    #[allow(clippy::type_complexity)]
    callbacks: Vec<Option<Box<dyn FnMut(Ref<T>)>>>,
}

impl<T> SharedState<T> {
    fn new(value: T) -> Self {
        Self {
            value,
            callbacks: vec![],
        }
    }
}

pub struct Source<T>(Rc<UnsafeCell<SharedState<T>>>);

assert_impl_all! {
    Source<String>:
        ops::Deref<Target = String>,
        fmt::Display,
        fmt::Debug,
        From<String>
}
assert_not_impl_any!(Source<()>: Clone);

impl<T> fmt::Debug for Source<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { (*self.0.get()).value.fmt(f) }
    }
}

impl<T> ops::Deref for Source<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &(*self.0.get()).value }
    }
}

impl<T> fmt::Display for Source<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { (*self.0.get()).value.fmt(f) }
    }
}

impl<T> From<T> for Source<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> Source<T> {
    pub fn new(value: T) -> Self {
        Self(Rc::new(UnsafeCell::new(SharedState::new(value))))
    }

    pub fn observer(&self) -> Observer<T> {
        Observer(Rc::downgrade(&self.0))
    }

    fn notify(&mut self) {
        unsafe {
            (*self.0.get()).callbacks.iter_mut().for_each(|f| {
                let v_ref = Ref(Rc::downgrade(&self.0));

                if let Some(f) = f {
                    f(v_ref);
                }
            })
        }
    }

    pub fn set(&mut self, value: T) {
        unsafe { (*self.0.get()).value = value };

        self.notify();
    }
}

pub struct Unsub<T>(usize, Weak<UnsafeCell<SharedState<T>>>);

impl<T> Unsubscribe for Unsub<T> {
    fn unsubscribe(self: Box<Self>) {
        if let Some(shared_state) = self.1.upgrade() {
            unsafe { (*shared_state.get()).callbacks[self.0] = None };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_source() {
        let s = Source::new(0);
        let s = Source::from(0);
        let s: Source<i32> = 0.into();
    }

    #[test]
    fn set_value() {
        let mut s = Source::new(0);

        let expected_v = Rc::new(RefCell::new(0));
        let count = Rc::new(RefCell::new(0));

        s.observer()
            .subscribe(cloned![[count, expected_v], move |v| {
                assert_eq!(*v, *expected_v.borrow());

                *count.borrow_mut() += 1;
            }]);

        assert_eq!(*count.borrow(), 1);

        *expected_v.borrow_mut() = 7;

        s.set(7);

        assert_eq!(*count.borrow(), 2);
    }

    #[test]
    fn map() {
        let mut s = Source::new(1);

        let expected_v = Rc::new(RefCell::new(2));
        let count = Rc::new(RefCell::new(0));

        s.observer().map(|v| *v * 2).subscribe(cloned![
            [count, expected_v],
            move |v| {
                assert_eq!(v, *expected_v.borrow());

                *count.borrow_mut() += 1;
            }
        ]);

        assert_eq!(*count.borrow(), 1);

        *expected_v.borrow_mut() = 14;

        s.set(7);

        assert_eq!(*count.borrow(), 2);
    }
}
