use std::{
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    future::Future,
    ops,
    pin::Pin,
    rc::{Rc, Weak},
    task::{Context, Poll, Wake, Waker},
};

use futures::Stream;

/// Source must:
/// - Hold refs to [`Observer`] wakers
#[derive(Educe)]
#[educe(Default(bound))]
pub struct Source<T> {
    shared_state: Rc<SharedState<T>>,
}

assert_not_impl_any!(Source<()>: Clone);
assert_impl_all!(Source<()>: ops::Deref);

impl<T> ops::Deref for Source<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // This is safe because the borrow checker makes sure we don't break
        // Rust's aliasing rules
        unsafe { &*self.shared_state.data.get() }
    }
}

/// Wakes up all the streams so they can notice the stream has ended.
impl<T> Drop for Source<T> {
    fn drop(&mut self) {
        trace!("dropping Source");

        self.shared_state
            .observer_wakers
            .borrow()
            .map
            .values()
            .for_each(|w| w.wake_by_ref());
    }
}

impl<T> From<T> for Source<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<T> Source<T> {
    pub fn new(data: T) -> Self {
        Self {
            shared_state: Rc::new(SharedState::new(data)),
        }
    }

    pub fn observer(&self) -> Observer<T> {
        let id = self.shared_state.observer_wakers.borrow_mut().get_id();

        Observer {
            id,
            shared_state: Rc::downgrade(&self.shared_state),
            changed: true,
        }
    }

    fn notify<O>(
        &mut self,
        shared_state: Rc<SharedState<T>>,
        set_fn: impl FnOnce(&mut T) -> O,
    ) -> impl Future<Output = O> {
        // This is a trick so we can move `set_fn` into an `FnMut` closure,
        // since `set_fn` is a `FnOnce`
        let set_fn = RefCell::new(Some(set_fn));
        let o = RefCell::new(None);

        futures::future::poll_fn(move |cx| {
            // Is this the first time we're being polled?
            let initial_poll = shared_state.source_fut_ref.borrow().is_none();

            // We should only get polled twice. Once to wake up all the observers,
            // and the second time once all observers are done reacting to the
            // change in data
            if initial_poll {
                // Update the data
                let set_fn = set_fn.borrow_mut().take().unwrap();

                // SAFETY:
                // This is safe because there is no other way to get a mutable borrow
                // to the inner data. The reference cannot escape the closure
                *o.borrow_mut() =
                    Some(unsafe { set_fn(&mut *shared_state.data.get()) });

                // Reset the task count
                *shared_state.completed_subscribers.borrow_mut() = 0;

                *shared_state.source_fut_ref.borrow_mut() =
                    Some(cx.waker().to_owned());

                // Wake up the party
                shared_state
                    .observer_wakers
                    .borrow()
                    .map
                    .values()
                    .for_each(|w| w.wake_by_ref());

                // Now we wait...
                Poll::Pending
            } else {
                // Remove the waker, as this is how we will know in the future
                // if we are being polled for the first or second time
                *shared_state.source_fut_ref.borrow_mut() = None;

                // And we're done!
                let o = o.borrow_mut().take().unwrap();
                Poll::Ready(o)
            }
        })
    }

    #[must_use = "observers will not react to changes until the future is polled"]
    pub fn set(&mut self, new_value: T) -> impl Future<Output = ()> {
        let shared_state = self.shared_state.clone();

        self.notify(shared_state, move |v| {
            *v = new_value;
        })
    }

    #[must_use = "observers will not react to changes until the future is polled"]
    pub fn set_with<O>(
        &mut self,
        f: impl FnOnce(&mut T) -> O,
    ) -> impl Future<Output = O> {
        let shared_state = self.shared_state.clone();

        self.notify(shared_state, |v| f(v))
    }
}

/// Observer must do the following:
/// - Must unregister waker when dropped
/// - Must yield first value when polled for the first time
/// - Must only yield subsecuent values when they change
/// - Mark itself as subscribed when polled, and unsubscribe when a non-pending
///   value is yielded. This is to handle the case of a user calling `poll_next`
///   only once, which would cause the subscription mechanism to leak, and the update
///   fn will never get to continue
pub struct Observer<T> {
    id: usize,
    shared_state: Weak<SharedState<T>>,
    changed: bool,
}

assert_impl_all!(Observer<()>: Clone, Stream);

/// Clones the observer, but with a new id and setting `changed` to true
/// so it can yield the first value.
impl<T> Clone for Observer<T> {
    fn clone(&self) -> Self {
        let id = if let Some(shared_state) = self.shared_state.upgrade() {
            shared_state.observer_wakers.borrow_mut().get_id()
        } else {
            0
        };

        Self {
            id,
            shared_state: self.shared_state.clone(),
            changed: true,
        }
    }
}

/// Removes self from [`SharedState`] observer_wakers.
impl<T> Drop for Observer<T> {
    fn drop(&mut self) {
        trace!("dropping Observer with id {}", self.id);

        if let Some(shared_state) = self.shared_state.upgrade() {
            shared_state
                .observer_wakers
                .borrow_mut()
                .map
                .remove(&self.id);
        }
    }
}

impl<T> Stream for Observer<T> {
    type Item = Ref<T>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        // Make sure the stream hasn't ended
        if let Some(shared_state) = self.shared_state.upgrade() {
            // The first thing we need to do is update the waker
            shared_state
                .observer_wakers
                .borrow_mut()
                .map
                .insert(self.id, cx.waker().to_owned());

            // Has our value changed since the last time we were polled?
            if self.changed {
                // We're done changing
                self.changed = false;

                Poll::Ready(Some(Ref(shared_state.data.clone())))
            } else {
                // We don't plan on being woken up, until the next value is available.
                // Even if this isn't true, and the user polled us twice, there's no
                // harm in yielding the current value.
                self.changed = true;

                // If there is no source future waker, this means we are going to initiate a
                // subscription. The way we know this, is because if source future waker exists,
                // this means we are currently being awaited for after a source change, and cannot
                // therefore create subscriptions, but rather react to them.
                if shared_state.source_fut_ref.borrow().is_none() {
                    *shared_state.subscriber_count.borrow_mut() += 1;
                }
                // Otherwise, we're going to mark this change as complete
                else {
                    *shared_state.completed_subscribers.borrow_mut() += 1;

                    if *shared_state.completed_subscribers.borrow()
                        == *shared_state.subscriber_count.borrow()
                    {
                        shared_state
                            .source_fut_ref
                            .borrow()
                            .as_ref()
                            .unwrap()
                            .wake_by_ref();
                    }
                }

                Poll::Pending
            }
        } else {
            Poll::Ready(None)
        }
    }
}

impl<T> Observer<T> {
    pub fn with<O>(&self, f: impl FnOnce(Option<&T>) -> O) -> O {
        if let Some(shared_state) = self.shared_state.upgrade() {
            // SAFETY:
            // This is safe because the reference cannot escape the closure.
            unsafe { f(Some(&*shared_state.data.get())) }
        } else {
            f(None)
        }
    }
}

pub struct Ref<T>(Rc<UnsafeCell<T>>);

assert_not_impl_any!(Ref<()>: Clone);
assert_impl_all!(Ref<()>: ops::Deref);

impl<T> ops::Deref for Ref<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY:
        // Since we own a strong reference to the data, the data is guaranteed
        // to not be dropped while we have a reference to the data.
        // We also guarantee that no mutability of the data is possible while we
        // hold said data, via the API design.
        //
        // This is, that any mutation to a `Source`, must be `.await`ed, this
        // future will wait for all `Observer`s to finish reacting to the change,
        // and only after this immutable period, can a `Source` once again
        // mutate the data.
        // This design is further enforced by promising to only run in a single-
        // threaded context.
        //
        // Furthermore, a `Ref<T>` can only be given out within a closure and
        // must **not** be clonable.

        // SAFETY:
        // This is safe because the only way to get a `Ref<T>` is when it is being
        // polled from a stream, and only by dereferencing it can you get an
        // immutable reference to the inner value
        unsafe { &*self.0.get() }
    }
}

#[derive(Educe)]
#[educe(Default(bound))]
struct SharedState<T> {
    data: Rc<UnsafeCell<T>>,
    /// For waking up the [`Source`] when all subscribers finish reacting to
    /// the change.
    source_fut_ref: RefCell<Option<Waker>>,
    /// For waking up the [`Observers`] when the [`Source`] is awaited.
    observer_wakers: RefCell<WakerMap>,
    /// To keep track of how many `completed_sbuscribers` are left before
    /// letting the [`Source`] know it's done waiting.
    subscriber_count: RefCell<usize>,
    /// To keep track of how many subscribers are left before letting
    /// the [`Source`] know it's done waiting.
    completed_subscribers: RefCell<usize>,
}

impl<T> SharedState<T> {
    fn new(data: T) -> Self {
        Self {
            data: Rc::new(UnsafeCell::new(data)),
            source_fut_ref: RefCell::new(None),
            observer_wakers: RefCell::new(WakerMap {
                map: HashMap::new(),
                next_id: 0,
            }),
            subscriber_count: RefCell::new(0),
            completed_subscribers: RefCell::new(0),
        }
    }
}

#[derive(Default)]
struct WakerMap {
    map: HashMap<usize, Waker>,
    next_id: usize,
}

impl WakerMap {
    fn get_id(&mut self) -> usize {
        let id = self.next_id;

        self.next_id += 1;

        id
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use futures::StreamExt;

    use super::*;
    use crate::utils::execute_async;

    #[test]
    fn source_deref() {
        let s = Source::new(7);

        assert_eq!(*s, 7);
    }

    #[tokio::test]
    #[ntest::timeout(1)]
    async fn observer_can_get_value() {
        let s = Source::new(7);

        let mut o = s.observer();

        let n = o.next().await.unwrap();

        assert_eq!(*n, 7);

        assert_eq!(*s.shared_state.subscriber_count.borrow(), 0);
        assert_eq!(*s.shared_state.completed_subscribers.borrow(), 0);
    }

    #[tokio::test]
    #[ntest::timeout(1)]
    async fn observers_can_subscribe() {
        let s = Source::new(7);

        execute_async(s.observer().for_each(|v| {
            assert_eq!(*v, 7);

            async {}
        }));

        execute_async(s.observer().for_each(|v| {
            assert_eq!(*v, 7);

            async {}
        }));

        execute_async(s.observer().for_each(|v| {
            assert_eq!(*v, 7);

            async {}
        }));

        execute_async(async move {
            panic!();

            assert_eq!(*s.shared_state.subscriber_count.borrow(), 3);
            assert_eq!(*s.shared_state.completed_subscribers.borrow(), 0);
        });
    }

    #[tokio::test]
    #[ntest::timeout(1)]
    async fn source_can_set() {
        execute_async(async {
            let s = Source::new(0);

            let expected_v = Rc::new(RefCell::new(0));

            execute_async(s.observer().for_each(|v| {
                assert_eq!(*v, 7);

                async {}
            }));

            assert_eq!(*s.shared_state.subscriber_count.borrow(), 1);
            assert_eq!(*s.shared_state.completed_subscribers.borrow(), 0);
        });
    }
}
