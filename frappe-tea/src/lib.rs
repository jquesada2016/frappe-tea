#![cfg_attr(target_arch = "wasm32", no_std)]
#![feature(once_cell)]

#[macro_use]
extern crate alloc;

#[macro_use]
extern crate async_trait;
#[macro_use]
#[allow(unused_imports)]
extern crate enum_dispatch;

#[macro_use]
#[allow(unused_macros)]
mod utils;

use alloc::{
    boxed::Box,
    rc::{Rc, Weak},
    string::{String, ToString},
    vec::Vec,
};
use core::{
    cell::RefCell,
    fmt::{Display, Pointer},
    future::Future,
    lazy::OnceCell,
};
#[cfg(target_arch = "wasm32")]
use gloo::utils::document;
use utils::execute_async;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{prelude::*, JsCast};

#[enum_dispatch]
trait Node<Msg> {
    fn children(&self) -> &Vec<DynNode<Msg>>;
}

#[enum_dispatch(Node, Display)]
pub enum DynNode<Msg> {
    NodeTree(NodeTree<Msg>),
}

impl<Msg> Display for DynNode<Msg> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NodeTree(n) => n.fmt(f),
        }
    }
}

pub trait IntoCmd<Msg> {
    fn into_cmd(self: Box<Self>) -> Box<dyn Cmd<Msg>>;
}

pub trait IntoNode<Msg> {}

trait DispatchMsg<Msg> {
    fn dispatch_msg(self: Rc<Self>, msg: Msg);
}

trait Runtime<Msg> {
    fn dispatch_msg(self: Rc<Self>, msg: Msg);

    fn perform_cmd(self: Rc<Self>, cmd: Box<dyn Cmd<Msg>>);
}

#[async_trait]
pub trait Cmd<Msg> {
    async fn execute(self: Box<Self>) -> Option<Msg>;
}

pub struct Element<M, Msg> {
    model: RefCell<M>,
    #[allow(clippy::type_complexity)]
    update_fn: fn(&mut M, Msg) -> Option<Box<dyn IntoCmd<Msg>>>,
}

impl<M, Msg> Element<M, Msg>
where
    M: 'static,
    Msg: 'static,
{
    #[allow(clippy::type_complexity)]
    pub fn new<F, C, Fut, N>(
        target: &str,
        initial_model: F,
        update: fn(&mut M, Msg) -> Option<Box<dyn IntoCmd<Msg>>>,
        view: fn(&M) -> Fut,
    ) -> Rc<Self>
    where
        F: FnOnce() -> (M, Option<Box<C>>),
        C: IntoCmd<Msg>,
        Fut: Future<Output = N>,
        N: IntoNode<Msg>,
    {
        let (model, initial_cmd) = initial_model();

        let this = Rc::new(Self {
            model: RefCell::new(model),
            update_fn: update,
        });

        let this_weak = Rc::downgrade(&this);

        render(target, view(&this.model.borrow()), this_weak);

        if let Some(cmd) = initial_cmd {
            this.clone().perform_cmd(cmd.into_cmd());
        }

        this
    }
}

impl<M, Msg> DispatchMsg<Msg> for Element<M, Msg>
where
    M: 'static,
    Msg: 'static,
{
    fn dispatch_msg(self: Rc<Self>, msg: Msg) {
        Runtime::dispatch_msg(self, msg);
    }
}

impl<M, Msg> Runtime<Msg> for Element<M, Msg>
where
    M: 'static,
    Msg: 'static,
{
    fn dispatch_msg(self: Rc<Self>, msg: Msg) {
        let cmd = (self.update_fn)(&mut self.model.borrow_mut(), msg);

        if let Some(cmd) = cmd {
            self.perform_cmd(cmd.into_cmd());
        }
    }

    fn perform_cmd(self: Rc<Self>, cmd: Box<dyn Cmd<Msg>>) {
        execute_async(async move {
            let msg = cmd.execute().await;

            if let Some(msg) = msg {
                Runtime::dispatch_msg(self, msg);
            }
        });
    }
}

fn render<Msg>(
    target: &str,
    children: impl Future<Output = impl IntoNode<Msg>>,
    dispatch_msg_weak: Weak<dyn DispatchMsg<Msg>>,
) {
    todo!()
}

pub enum NodeTree<Msg> {
    Component {
        /// Reference to the message dispatcher
        msg_dispatcher: OnceCell<Weak<dyn DispatchMsg<Msg>>>,
        /// Marks the start of a component.
        opening_comment: Comment,
        /// Component name
        name: &'static str,
        children: Vec<DynNode<Msg>>,
        /// Marks the end of a component.
        closing_comment: Comment,
    },
    Tag {
        msg_dispatcher: OnceCell<Weak<dyn DispatchMsg<Msg>>>,
        name: String,
    },
    Text {
        text: String,
        #[cfg(target_arch = "wasm32")]
        node: web_sys::Text,
    },
}

impl<Msg> Display for NodeTree<Msg> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Component { name, .. } => {
                f.write_fmt(format_args!("<{name}>"))?;

                for child in self.children().iter() {
                    child.fmt(f)?;
                }

                f.write_fmt(format_args!("</{name}>"))?;

                Ok(())
            }
            Self::Text { text, .. } => text.fmt(f),
            _ => todo!(),
        }
    }
}

impl<Msg> Node<Msg> for NodeTree<Msg> {
    fn children(&self) -> &Vec<DynNode<Msg>> {
        match self {
            Self::Component { children, .. } => children,
            Self::Text { .. } => panic!("text nodes cannot have children"),
            _ => todo!(),
        }
    }
}

impl<Msg> NodeTree<Msg> {
    fn new_component(name: &'static str) -> Self {
        #[cfg(target_arch = "wasm32")]
        let (opening_comment, closing_comment) = {
            (
                document()
                    .create_comment(&format!(" <{}> ", name))
                    .unchecked_into(),
                document()
                    .create_comment(&format!(" <{} /> ", name))
                    .unchecked_into(),
            )
        };

        Self::Component {
            msg_dispatcher: OnceCell::new(),
            opening_comment: Comment {
                #[cfg(target_arch = "wasm32")]
                node: opening_comment,
            },
            closing_comment: Comment {
                #[cfg(target_arch = "wasm32")]
                node: closing_comment,
            },
            name,
            children: vec![],
        }
    }

    fn new_text(text: impl ToString) -> Self {
        let text = text.to_string();

        #[cfg(target_arch = "wasm32")]
        let node = document().create_text_node(&text);

        Self::Text {
            text,
            #[cfg(target_arch = "wasm32")]
            node,
        }
    }
}

pub struct Comment {
    #[cfg(target_arch = "wasm32")]
    node: web_sys::Node,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
}
