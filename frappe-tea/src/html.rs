use crate::{BoxNode, IntoNode, Node, NodeTree};
use futures::Stream;
use paste::paste;
#[cfg(feature = "ssr")]
use serde::{Deserialize, Serialize};
use std::{
    cell::{Ref, RefCell, RefMut},
    fmt,
    marker::PhantomData,
    ops,
    rc::Rc,
    sync::{Arc, Mutex},
};
use wasm_bindgen::prelude::*;

mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

#[async_trait(?Send)]
pub trait Html<Msg>: sealed::Sealed + Node<Msg> {
    fn text(&mut self, text: impl ToString) -> &mut Self {
        todo!()
    }

    fn dyn_text<St, O>(
        &mut self,
        text_stream: St,
        text_fn: impl FnMut(St::Item) -> O,
    ) -> &mut Self
    where
        St: Stream,
        O: ToString,
    {
        todo!()
    }

    async fn child<'a>(&mut self, child: impl IntoNode<Msg> + 'a) -> &mut Self {
        self.append_child(child.into_node().await);

        self
    }

    fn dyn_child<St, F, C>(&mut self, stream: St, child_fn: F) -> &mut Self
    where
        St: Stream,
        F: FnMut(St::Item) -> Option<C>,
        C: IntoNode<Msg>,
    {
        todo!()
    }

    fn attr(&mut self, name: impl ToString, value: impl ToString) -> &mut Self {
        todo!()
    }

    fn dyn_attr<St, I>(
        &mut self,
        name: impl ToString,
        value_stream: St,
    ) -> &mut Self
    where
        St: Stream<Item = Option<I>>,
        I: ToString,
    {
        todo!()
    }

    fn attr_bool(&mut self, name: impl ToString) -> &mut Self {
        todo!()
    }

    fn dyn_attr_bool<St>(
        &mut self,
        name: impl ToString,
        bool_stream: St,
    ) -> &mut Self
    where
        St: Stream<Item = bool>,
    {
        todo!()
    }

    fn bind_attr<St, V, F>(
        &mut self,
        attr_name: impl ToString,
        value_stream: St,
        to_msg: F,
    ) -> &mut Self
    where
        St: Stream<Item = Option<V>>,
        V: ToString,
        F: Fn(Option<String>) -> Msg,
    {
        todo!()
    }

    #[cfg(feature = "ssr")]
    fn prop(
        &mut self,
        name: impl ToString,
        value: impl Into<JsValue> + Serialize + for<'de> Deserialize<'de>,
    ) -> &mut Self {
        todo!()
    }

    #[cfg(not(feature = "ssr"))]
    fn prop(
        &mut self,
        name: impl ToString,
        value: impl Into<JsValue>,
    ) -> &mut Self {
        todo!()
    }

    #[cfg(feature = "ssr")]
    fn dyn_prop<St, I>(
        &mut self,
        name: impl ToString,
        value_stream: St,
    ) -> &mut Self
    where
        St: Stream<Item = Option<I>>,
        I: Into<JsValue> + Serialize + for<'de> Deserialize<'de>,
    {
        todo!()
    }

    #[cfg(not(feature = "ssr"))]
    fn dyn_prop<St, I>(
        &mut self,
        name: impl ToString,
        value_stream: St,
    ) -> &mut Self
    where
        St: Stream<Item = Option<I>>,
        I: Into<JsValue>,
    {
        todo!()
    }

    fn bind_prop_with_event<St, I, F>(
        &mut self,
        prop_name: impl ToString,
        event: impl ToString,
        value_stream: St,
        to_msg: F,
    ) -> &mut Self
    where
        St: Stream<Item = I>,
        I: Into<JsValue>,
        F: Fn(JsValue) -> Msg,
    {
        todo!()
    }

    fn class(&mut self, class_name: impl ToString) -> &mut Self {
        todo!()
    }

    fn class_bool(
        &mut self,
        class_name: impl ToString,
        expr: bool,
    ) -> &mut Self {
        todo!()
    }

    fn dyn_class<St>(
        &mut self,
        class_name: impl ToString,
        bool_stream: St,
    ) -> &mut Self
    where
        St: Stream<Item = bool>,
    {
        todo!()
    }

    fn on(
        &mut self,
        event_type: impl Into<std::borrow::Cow<'static, str>>,
        mut callback: impl FnMut(&web_sys::Event) -> Option<Msg>,
    ) -> &mut Self {
        todo!()
    }

    fn on_with_options(
        &mut self,
        event_type: impl Into<std::borrow::Cow<'static, str>>,
        mut callback: impl FnMut(&web_sys::Event) -> Option<Msg>,
        options: gloo::events::EventListenerOptions,
    ) -> &mut Self {
        todo!()
    }
}

pub struct HtmlElement<E, Msg> {
    _element: PhantomData<E>,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    node: Option<NodeTree<Msg>>,
}

impl<E, Msg> sealed::Sealed for HtmlElement<E, Msg> where E: sealed::Sealed {}

impl<E, Msg> Html<Msg> for HtmlElement<E, Msg> where E: sealed::Sealed {}

impl<E, Msg> Node<Msg> for HtmlElement<E, Msg>
where
    E: sealed::Sealed,
{
    fn node(&self) -> &NodeTree<Msg> {
        &self
            .node
            .as_ref()
            .expect("attempted to get node after calling `into_node()`")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn children_rc(&self) -> Arc<Mutex<Vec<BoxNode<Msg>>>> {
        self.node
            .as_ref()
            .expect_throw(
                "attempted to get children after calling `into_node()`",
            )
            .children_rc()
    }

    #[cfg(target_arch = "wasm32")]
    fn children_rc(&self) -> Rc<RefCell<Vec<BoxNode<Msg>>>> {
        self.node
            .as_ref()
            .expect_throw(
                "attempted to get children after calling `into_node()`",
            )
            .children_rc()
    }

    fn children<'a>(
        &'a self,
    ) -> Box<dyn ops::Deref<Target = Vec<BoxNode<Msg>>> + 'a> {
        self.node
            .as_ref()
            .expect_throw(
                "attempted to get children after calling `into_node()`",
            )
            .children()
    }

    fn children_mut(
        &mut self,
    ) -> Box<dyn ops::DerefMut<Target = Vec<BoxNode<Msg>>>> {
        self.node
            .as_mut()
            .expect_throw(
                "attempted to get children after calling `into_node()`",
            )
            .children_mut()
    }

    fn append_child(&mut self, child: BoxNode<Msg>) {
        self.node
            .as_mut()
            .expect_throw(
                "attempted to append child after calling `into_node()`",
            )
            .append_child(child);
    }
}

#[async_trait(?Send)]
impl<E, Msg> IntoNode<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed + 'static,
{
    async fn into_node(self) -> BoxNode<Msg> {
        self.node
            .expect_throw("called `into_node()` more than once")
            .into_node()
            .await
    }
}

#[async_trait(?Send)]
impl<E, Msg> IntoNode<Msg> for &mut HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed + 'static,
{
    async fn into_node(self) -> BoxNode<Msg> {
        self.node
            .take()
            .expect_throw("called `into_node()` more than once")
            .into_node()
            .await
    }
}

impl<E, Msg> HtmlElement<E, Msg>
where
    E: sealed::Sealed,
{
    fn new(_element: E, name: &'static str) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::new_element(name)),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn from_raw_node(_element: E, node: web_sys::Node) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::from_raw_node(node)),
        }
    }
}

macro_rules! generate_html_tags {
    ($($tag:ident),* $(,)?) => {
        paste! {
            $(
                #[derive(Copy, Clone, Debug)]
                pub struct [<$tag:camel>];

                impl sealed::Sealed for [<$tag:camel>] {}

                impl fmt::Display for [<$tag:camel>] {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(stringify!($tag))
                    }
                }

                impl<Msg> PartialEq<BoxNode<Msg>> for [<$tag:camel>] {
                    fn eq(&self, rhs: &BoxNode<Msg>) -> bool {
                        match rhs.node() {
                            NodeTree::Tag { name, .. } => *name == self.to_string(),
                            _ => false,
                        }
                    }
                }

                impl<Msg> PartialEq<[<$tag:camel>]> for BoxNode<Msg> {
                    fn eq(&self, rhs: &[<$tag:camel>]) -> bool {
                        match self.node() {
                            NodeTree::Tag { name, .. } => *name == rhs.to_string(),
                            _ => false,
                        }
                    }
                }

                pub fn $tag<Msg>() -> HtmlElement<[<$tag:camel>], Msg> {
                    HtmlElement::new([<$tag:camel>], stringify!($tag))
                }
            )*
        }
    };
}

generate_html_tags![body, div, button, h1, h2, h3];

#[cfg(test)]
mod tests {
    use futures::executor::block_on;

    use super::*;

    #[test]
    fn can_append_children() {
        let mut r: HtmlElement<Div, ()> = div();

        r.append_child(Box::new(h1()));

        assert_eq!(r.children().len(), 1);

        assert_eq!(r.children()[0], H1);
    }

    #[test]
    fn el_eq_el_node() {
        let node: BoxNode<()> = block_on(div().into_node());

        assert_eq!(&Div, &node);
    }

    #[test]
    fn el_not_eq_el_node() {
        let node: BoxNode<()> = block_on(div().into_node());

        assert_ne!(&H1, &node);
    }
}
