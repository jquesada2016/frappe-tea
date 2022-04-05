#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::RefCell,
    future::Future,
    lazy::SyncOnceCell,
    marker::PhantomData,
    rc::Rc,
    sync::{Arc, Mutex},
};

// ==============================================================
//                      Traits
// ==============================================================

/// Trait that must be implemented by a type which wishes to be used as a command.
///
/// Commands are intended to be the primary way of performing side effects.
/// Keeping true to this rule will allow your apps to remain fully testable,
/// scalable, and above all, reliable.
#[async_trait]
pub trait Cmd<Msg>: Send {
    /// Executes a [`Cmd`] and passes it's resulting message, if any, to
    /// `update`.
    async fn perform_cmd(self: Box<Self>) -> Option<Msg>;
}

pub trait DispatchMsg<Msg> {
    fn dispatch_msg(self: Rc<Self>, msg: Msg);
}

#[async_trait]
pub trait Node<Msg> {}

#[async_trait]
trait Runtime<Msg> {
    /// Executes a [`Cmd`] and passes it's resulting message, if any, to
    /// `update`.
    async fn perform_cmd(&self, cmd: Box<dyn Cmd<Msg>>);

    /// Sends a message to `update`.
    async fn dispatch_msg(&self, msg: Msg);
}

// ==============================================================
//                    Structs and Impls
// ==============================================================

/// Represents a single app element instance which cannot perform navigation.
///
/// This is the primary type useful in SSR contexts.

pub struct AppElement<M, U, Msg>(Arc<Mutex<AppEl<M, U, Msg>>>);

struct AppEl<M, U, Msg> {
    model: RefCell<Option<M>>,
    update: U,
    _root: NodeTree<Msg>,
    _phantom: PhantomData<Msg>,
}

/// SAFETY:
/// This is safe because [`NodeTree`], which is potentially `!Sync`, will never be accessed.
/// It is simply there to keep the node tree from being prematurely dropped. There is also
/// no way to construct or access `Msg`, so no unsafety here.
unsafe impl<M, U, Msg> Sync for AppEl<M, U, Msg>
where
    Msg: Send,
    U: Sync,
{
}

assert_impl_all!(AppElement<(), fn(&mut ()) -> Option<Box<dyn Cmd<()>>>, ()>: Send, Sync);

#[async_trait]
impl<M, U, Msg, Fut> Runtime<Msg> for AppEl<M, U, Msg>
where
    M: Send,
    U: Fn(&mut M, Msg) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = Option<Box<dyn Cmd<Msg>>>> + Send,
    Msg: Send + 'static,
{
    async fn dispatch_msg(&self, msg: Msg) {
        // Take the model
        let mut model = { self.model.borrow_mut().take().unwrap() };

        let cmd = (self.update)(&mut model, msg).await;

        // Return the model
        *self.model.borrow_mut() = Some(model);

        if let Some(cmd) = cmd {
            self.perform_cmd(cmd).await;
        }
    }

    async fn perform_cmd(&self, cmd: Box<dyn Cmd<Msg>>) {
        let msg = cmd.perform_cmd().await;

        if let Some(msg) = msg {
            self.dispatch_msg(msg).await;
        }
    }
}

pub struct NodeTree<Msg> {
    _root: SyncOnceCell<bool>,
    _phantom: PhantomData<Msg>,
}

assert_impl_all!(AppElement<(), fn(&mut ()) -> Option<Box<dyn Cmd<()>>>, ()>: Send);
