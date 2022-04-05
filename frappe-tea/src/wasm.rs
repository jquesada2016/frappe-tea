#![cfg(target_arch = "wasm32")]

use std::{
    cell::RefCell, future::Future, lazy::OnceCell, marker::PhantomData, rc::Rc,
};

// ==============================================================
//                      Traits
// ==============================================================

/// Trait that must be implemented by a type which wishes to be used as a command.
///
/// Commands are intended to be the primary way of performing side effects.
/// Keeping true to this rule will allow your apps to remain fully testable,
/// scalable, and above all, reliable.
#[async_trait(?Send)]
pub trait Cmd<Msg> {
    /// Executes a [`Cmd`] and passes it's resulting message, if any, to
    /// `update`.
    async fn perform_cmd(self: Box<Self>) -> Option<Msg>;
}

pub trait DispatchMsg<Msg> {
    fn dispatch_msg(self: Rc<Self>, msg: Msg);
}

#[async_trait(?Send)]
pub trait Node<Msg> {}

#[async_trait(?Send)]
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

pub struct AppElement<M, U, Msg>(Rc<AppEl<M, U, Msg>>);

struct AppEl<M, U, Msg> {
    model: RefCell<Option<M>>,
    update: U,
    _root: NodeTree<Msg>,
    _phantom: PhantomData<Msg>,
}

#[async_trait(?Send)]
impl<M, U, Msg, Fut> Runtime<Msg> for AppEl<M, U, Msg>
where
    U: Fn(&mut M, Msg) -> Fut + 'static,
    Fut: Future<Output = Option<Box<dyn Cmd<Msg>>>>,
    Msg: 'static,
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
    _root: OnceCell<bool>,
    _phantom: PhantomData<Msg>,
}
