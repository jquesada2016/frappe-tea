#![feature(once_cell)]
#![allow(warnings)]

#[macro_use]
extern crate async_trait;
#[macro_use]
#[allow(unused_imports)]
extern crate enum_dispatch;
#[macro_use]
extern crate typed_builder;

#[macro_use]
#[allow(unused_macros)]
mod utils;
mod components;
pub mod html;
mod testing;
#[cfg(target_arch = "wasm32")]
use gloo::utils::document;
use std::{
    cell::RefCell,
    fmt,
    future::Future,
    lazy::OnceCell,
    marker::PhantomData,
    ops,
    rc::{Rc, Weak},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};
use utils::execute_async;
use wasm_bindgen::{prelude::*, JsCast};

/// This module exposes useful constants regarding the environment of the currently running
/// app.
pub mod env {
    use wasm_bindgen::prelude::*;

    /// Constant specifying if the app was build in development mode (`debug_assertions`).
    pub const DEV: bool = cfg!(debug_assertions);

    /// Helper function to determin if the app is currently running
    /// in the browser.
    ///
    /// This value is `false` when SSR is being performed, and code is
    /// therefore running on the server, beit a wasm target, such as `node.js`
    /// or `Deno`, or any non-wasm target.
    pub fn is_browser() -> bool {
        // Can't use web_sys::window() or web_sys::document() because it throws
        // error due to window not being defined to check to see if the window exists...
        #[cfg(target_arch = "wasm32")]
        {
            let global = js_sys::global();

            let window =
                js_sys::Reflect::get(&global, &"window".into()).unwrap_throw();

            if window.is_undefined() {
                return false;
            }

            let document =
                js_sys::Reflect::get(&window, &"window".into()).unwrap_throw();

            return !document.is_undefined();
        };

        #[cfg(not(target_arch = "wasm32"))]
        return false;
    }
}

#[doc(hidden)]
pub mod __private_internals__ {
    pub use typed_builder::TypedBuilder;
}

pub mod prelude {
    pub use super::{
        components::*,
        html::{self, Html},
        *,
    };
}

pub type BoxNode<Msg> = Box<dyn Node<Msg>>;

pub trait Node<Msg> {
    fn node(&self) -> &NodeTree<Msg>;

    fn children(&self) -> RwLockReadGuard<Vec<BoxNode<Msg>>>;

    fn children_mut(&mut self) -> RwLockWriteGuard<Vec<BoxNode<Msg>>>;

    fn append_child(&mut self, child: BoxNode<Msg>) {
        #[cfg(target_arch = "wasm32")]
        if env::is_browser() {
            match self.node() {
                NodeTree::Component { .. } => { /* Nothing to do here */ }
                NodeTree::Tag {
                    node: Some(parent), ..
                } => match child.node() {
                    NodeTree::Component {
                        opening_comment,
                        closing_comment,
                        children,
                        ..
                    } => {
                        // First, insert the opening comment node
                        parent
                            .append_child(
                                &opening_comment.node.as_ref().unwrap_throw(),
                            )
                            .unwrap_throw();

                        // Next, add all children
                        children
                            .read()
                            .unwrap_throw()
                            .recursively_append_children_to_dom(parent);

                        // Lastly, insert closing comment node
                        parent
                            .append_child(
                                &closing_comment.node.as_ref().unwrap_throw(),
                            )
                            .unwrap_throw();
                    }
                    NodeTree::Tag {
                        node: Some(child), ..
                    } => {
                        parent.append_child(child).unwrap_throw();
                    }
                    NodeTree::Text {
                        node: Some(child), ..
                    } => {
                        parent.append_child(child).unwrap_throw();
                    }
                    _ => unreachable!(),
                },
                NodeTree::Text { .. } => {
                    panic!("text nodes cannot have children")
                }
                _ => unreachable!(),
            }
        }

        self.children_mut().push(child);
    }

    fn clear_children(&mut self) {
        self.children_mut().clear();
    }
}

impl<Msg> ops::Deref for dyn Node<Msg> {
    type Target = NodeTree<Msg>;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl<Msg> fmt::Debug for dyn Node<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node().fmt(f)
    }
}

impl<Msg> fmt::Display for dyn Node<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node().fmt(f)
    }
}

pub trait IntoCmd<Msg> {
    fn into_cmd(self: Box<Self>) -> Box<dyn Cmd<Msg>>;
}

#[cfg(target_arch = "wasm32")]
trait NodeVecExt {
    /// Helper function for recursively appending children to a component.
    ///
    /// It is intended to recurse it's children until a `tag` or `text` node
    /// is encountered.
    fn recursively_append_children_to_dom(&self, target: &web_sys::Node);
}

#[cfg(target_arch = "wasm32")]
impl<Msg> NodeVecExt for [BoxNode<Msg>] {
    fn recursively_append_children_to_dom(&self, target: &web_sys::Node) {
        for child in self {
            match child.node() {
                NodeTree::Tag { node, .. } => {
                    target
                        .append_child(node.as_ref().unwrap_throw())
                        .unwrap_throw();
                }
                NodeTree::Text { node, .. } => {
                    target
                        .append_child(node.as_ref().unwrap_throw())
                        .unwrap_throw();
                }
                NodeTree::Component {
                    opening_comment,
                    children,
                    closing_comment,
                    ..
                } => {
                    // First, insert opening comment node
                    target
                        .append_child(
                            &opening_comment.node.as_ref().unwrap_throw(),
                        )
                        .unwrap_throw();

                    // Add children
                    children
                        .read()
                        .unwrap_throw()
                        .recursively_append_children_to_dom(target);

                    // Lastly, add closing comment node
                    target
                        .append_child(
                            &closing_comment.node.as_ref().unwrap_throw(),
                        )
                        .unwrap_throw();
                }
            }
        }
    }
}

pub trait IntoNode<Msg> {
    fn into_node(self) -> BoxNode<Msg>;
}

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

#[cfg(target_arch = "wasm32")]
pub struct Element<M, Msg, UF> {
    model: RefCell<M>,
    #[allow(clippy::type_complexity)]
    update_fn: UF,
    root_node: OnceCell<BoxNode<Msg>>,
    _msg_type: PhantomData<Msg>,
}

#[cfg(target_arch = "wasm32")]
impl<M, Msg, UF> Element<M, Msg, UF>
where
    M: 'static,
    Msg: 'static,
    UF: Fn(&mut M, Msg) -> Option<Box<dyn IntoCmd<Msg>>> + 'static,
{
    #[allow(clippy::type_complexity)]
    pub async fn new<Fut, const N: usize>(
        target: &str,
        initial_model: impl FnOnce() -> (M, Option<Box<dyn IntoCmd<Msg>>>),
        update: UF,
        view: impl FnOnce(&M) -> Fut,
    ) -> Rc<Self>
    where
        Fut: Future<Output = [BoxNode<Msg>; N]>,
    {
        let (model, initial_cmd) = initial_model();

        let this = Rc::new(Self {
            model: RefCell::new(model),
            update_fn: update,
            root_node: OnceCell::new(),
            _msg_type: PhantomData::default(),
        });

        let this_weak = Rc::downgrade(&this);

        let children = view(&this.model.borrow()).await;

        let root_node = render(target, children, this_weak);

        this.root_node.set(root_node).unwrap_throw();

        if let Some(cmd) = initial_cmd {
            this.clone().perform_cmd(cmd.into_cmd());
        }

        this
    }

    pub fn root_node(&self) -> &BoxNode<Msg> {
        &self.root_node.get().unwrap_throw()
    }
}

#[cfg(target_arch = "wasm32")]
impl<M, Msg, UF> DispatchMsg<Msg> for Element<M, Msg, UF>
where
    M: 'static,
    Msg: 'static,
    UF: Fn(&mut M, Msg) -> Option<Box<dyn IntoCmd<Msg>>> + 'static,
{
    fn dispatch_msg(self: Rc<Self>, msg: Msg) {
        Runtime::dispatch_msg(self, msg);
    }
}

#[cfg(target_arch = "wasm32")]
impl<M, Msg, UF> Runtime<Msg> for Element<M, Msg, UF>
where
    M: 'static,
    Msg: 'static,
    UF: Fn(&mut M, Msg) -> Option<Box<dyn IntoCmd<Msg>>> + 'static,
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

#[cfg(target_arch = "wasm32")]
fn render<Msg, const N: usize>(
    target: &str,
    children: [BoxNode<Msg>; N],
    dispatch_msg_weak: Weak<dyn DispatchMsg<Msg>>,
) -> BoxNode<Msg>
where
    Msg: 'static,
{
    assert!(
        env::is_browser(),
        "this render method is intended to noly work on browsers"
    );

    // First, get the target node
    let target = document()
        .query_selector(target)
        .expect_throw(&format!("failed to query `{target}`"))
        .expect_throw(&format!(
            "failed to find target node with query `{target}`"
        ))
        .unchecked_into();

    // Intern the node into an element
    let mut target = NodeTree::<Msg>::from_raw_node(target);

    // Set target as root
    match &mut target {
        NodeTree::Tag { root, .. } => *root = true,
        _ => unreachable!(),
    }

    // Insert children
    for child in children {
        target.append_child(child);
    }

    target.into_node()
}

#[cfg(not(target_arch = "wasm32"))]
fn render<Msg>(
    children: impl Future<Output = impl IntoNode<Msg>>,
    dispatch_msg_weak: Weak<dyn DispatchMsg<Msg>>,
) -> BoxNode<Msg>
where
    Msg: 'static,
{
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
        children: Arc<RwLock<Vec<BoxNode<Msg>>>>,
        /// Marks the end of a component.
        closing_comment: Comment,
    },
    Tag {
        /// Used to prevent the root node from being removed from the DOM
        /// when dropped.
        root: bool,
        msg_dispatcher: OnceCell<Weak<dyn DispatchMsg<Msg>>>,
        name: String,
        /// Optional because we might be running outside the browser.
        #[cfg(target_arch = "wasm32")]
        node: Option<web_sys::Node>,
        children: Arc<RwLock<Vec<BoxNode<Msg>>>>,
    },
    Text {
        text: String,
        #[cfg(target_arch = "wasm32")]
        /// Optional because we might be running outside the browser.
        node: Option<web_sys::Text>,
    },
}

impl<Msg> fmt::Debug for NodeTree<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text { text, .. } => text.fmt(f),
            Self::Tag { name, children, .. } => f
                .debug_struct("Tag")
                .field("name", name)
                .field("children", children)
                .finish(),
            Self::Component { name, children, .. } => f
                .debug_struct("Component")
                .field("name", name)
                .field("children", children)
                .finish(),
        }
    }
}

impl<Msg> fmt::Display for NodeTree<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Component { name, .. } => {
                // Opening comment node
                f.write_fmt(format_args!(r#"<template></template>"#))?;

                for child in self.children().iter() {
                    <Box<dyn Node<Msg>> as fmt::Display>::fmt(child, f)?;
                }

                // Closing comment node
                f.write_fmt(format_args!(r#"<template></template>"#))?;

                Ok(())
            }
            Self::Tag { name, children, .. } => {
                f.write_fmt(format_args!("<{name}>"))?;

                for child in self.children().iter() {
                    <Box<dyn Node<Msg>> as fmt::Display>::fmt(child, f)?;
                }

                f.write_fmt(format_args!("</{name}>"))?;

                Ok(())
            }
            Self::Text { text, .. } => text.fmt(f),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<Msg> Drop for NodeTree<Msg> {
    fn drop(&mut self) {
        match self {
            Self::Tag { root, node, .. } => {
                if !*root {
                    if let Some(node) = node {
                        node.unchecked_ref::<web_sys::Element>().remove();
                    }
                }
            }
            _ => {}
        }
    }
}

impl<Msg> IntoNode<Msg> for NodeTree<Msg>
where
    Msg: 'static,
{
    fn into_node(self) -> BoxNode<Msg> {
        Box::new(self)
    }
}

impl<Msg> Node<Msg> for NodeTree<Msg> {
    fn node(&self) -> &NodeTree<Msg> {
        self
    }

    fn children(&self) -> RwLockReadGuard<Vec<BoxNode<Msg>>> {
        match self {
            Self::Component { children, .. } => children.read().unwrap_throw(),
            Self::Tag { children, .. } => children.read().unwrap_throw(),
            Self::Text { .. } => panic!("text nodes cannot have children"),
        }
    }

    fn children_mut(&mut self) -> RwLockWriteGuard<Vec<BoxNode<Msg>>> {
        match self {
            Self::Component { children, .. } => children.write().unwrap_throw(),
            Self::Tag { children, .. } => children.write().unwrap_throw(),
            Self::Text { .. } => panic!("text nodes cannot have children"),
        }
    }
}

impl<Msg> NodeTree<Msg> {
    fn new_component(name: &'static str) -> Self {
        #[cfg(target_arch = "wasm32")]
        let (opening_comment, closing_comment) = if env::is_browser() {
            (
                Some(
                    document()
                        .create_comment(&format!(" <{}> ", name))
                        .unchecked_into(),
                ),
                Some(
                    document()
                        .create_comment(&format!(" <{} /> ", name))
                        .unchecked_into(),
                ),
            )
        } else {
            (None, None)
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
            children: Arc::new(RwLock::new(vec![])),
        }
    }

    #[track_caller]
    fn new_element(name: &'static str) -> Self {
        #[cfg(target_arch = "wasm32")]
        let node = if env::is_browser() {
            Some(
                gloo::utils::document()
                    .create_element(name)
                    .expect_throw(&format!(
                        "failed to create element `{}`",
                        name
                    ))
                    .unchecked_into(),
            )
        } else {
            None
        };

        Self::Tag {
            root: false,
            msg_dispatcher: OnceCell::new(),
            name: name.to_string(),
            #[cfg(target_arch = "wasm32")]
            node,
            children: Arc::new(RwLock::new(vec![])),
        }
    }

    fn new_text(text: impl ToString) -> Self {
        let text = text.to_string();

        #[cfg(target_arch = "wasm32")]
        let node = if env::is_browser() {
            Some(document().create_text_node(&text))
        } else {
            None
        };

        Self::Text {
            text,
            #[cfg(target_arch = "wasm32")]
            node,
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn from_raw_node(node: web_sys::Node) -> Self {
        // I'm assuming that if you managed to get your hands on a Node, it's
        // because we're running in a browser-like environment...
        let name = node.node_name().to_lowercase();

        Self::Tag {
            root: false,
            msg_dispatcher: OnceCell::new(),
            name,
            node: Some(node),
            children: Arc::new(RwLock::new(vec![])),
        }
    }
}

pub struct Comment {
    /// Optional because we might be running outside the browser.
    #[cfg(target_arch = "wasm32")]
    node: Option<web_sys::Node>,
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn node_can_add_child() {
        let mut r = NodeTree::<()>::new_component("Root");
        let child = NodeTree::<()>::new_component("Child");

        r.append_child(Box::new(child));

        assert_eq!(r.children().len(), 1);
    }

    #[test]
    fn node_can_clear_children() {
        let mut r = NodeTree::<()>::new_component("Root");
        let child = NodeTree::<()>::new_component("Child");

        r.append_child(Box::new(child));

        r.clear_children();

        assert!(r.children().is_empty());
    }
}
