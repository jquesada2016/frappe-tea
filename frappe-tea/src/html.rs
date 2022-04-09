use crate::{ChildrenMut, ChildrenRef, Id, IntoNode, Node, NodeKind};
use paste::paste;
use sealed::*;
use std::{fmt, future::Future, marker::PhantomData};

use crate::{Context, DynNode, NodeTree};

mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

#[async_trait]
pub trait Html<Msg>: Sealed + Node<Msg>
where
    Msg: 'static,
{
    async fn text<'a>(
        &mut self,
        text: impl ToString + Send + Sync + 'a,
    ) -> &mut Self {
        self.append_child(
            NodeTree::new_text(&text.to_string()).into_node().await,
        );

        self
    }

    async fn child<'a, F>(
        &mut self,
        child_fn: impl FnOnce(Context<Msg>) -> F + Send + Sync + 'a,
    ) -> &mut Self
    where
        F: Future<Output = DynNode<Msg>> + Send,
    {
        let index = self.children().len();
        let cx = self.cx();

        let mut child_cx = Context {
            msg_dispatcher: cx.msg_dispatcher.clone(),
            ..Default::default()
        };

        child_cx.id._set_id(&cx.id, index);

        let child = child_fn(child_cx).await;

        self.append_child(child);

        self
    }
}

pub struct HtmlElement<E, Msg> {
    _element: PhantomData<E>,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    node: Option<NodeTree<Msg>>,
}

impl<E, Msg> sealed::Sealed for HtmlElement<E, Msg> where E: sealed::Sealed {}

impl<E, Msg> Html<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed,
{
}

#[async_trait]
impl<E, Msg> IntoNode<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed + Send + 'static,
{
    async fn into_node(self) -> DynNode<Msg> {
        self.node
            .expect("called `into_node()` more than once")
            .into_node()
            .await
    }
}

#[async_trait]
impl<E, Msg> IntoNode<Msg> for &mut HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed + Send + Sync + 'static,
{
    async fn into_node(self) -> DynNode<Msg> {
        self.node
            .take()
            .expect("called `into_node()` more than once")
            .into_node()
            .await
    }
}

impl<E, Msg> Node<Msg> for HtmlElement<E, Msg>
where
    E: sealed::Sealed,
{
    fn node(&self) -> &NodeKind {
        self.node
            .as_ref()
            .expect("attempted to use node interface after calling `into_node`")
            .node()
    }

    fn node_mut(&mut self) -> &mut NodeKind {
        self.node
            .as_mut()
            .expect("attempted to use node interface after calling `into_node`")
            .node_mut()
    }

    fn cx(&self) -> &Context<Msg> {
        self.node
            .as_ref()
            .expect("attempted to use node interface after calling `into_node`")
            .cx()
    }

    fn set_ctx(&mut self, cx: Context<Msg>) {
        self.node
            .as_mut()
            .expect("attempted to use node interface after calling `into_node`")
            .set_ctx(cx)
    }

    fn children(&self) -> ChildrenRef<Msg> {
        self.node
            .as_ref()
            .expect("attempted to use node interface after calling `into_node`")
            .children()
    }

    fn children_mut(&mut self) -> ChildrenMut<Msg> {
        self.node
            .as_mut()
            .expect("attempted to use node interface after calling `into_node`")
            .children_mut()
    }

    #[track_caller]
    fn append_child(&mut self, child: DynNode<Msg>) {
        self.node
            .as_mut()
            .expect("attempted to use node interface after calling `into_node`")
            .append_child(child)
    }

    fn clear_children(&mut self) {
        self.node
            .as_mut()
            .expect("attempted to use node interface after calling `into_node`")
            .clear_children()
    }
}

impl<E, Msg> HtmlElement<E, Msg>
where
    E: sealed::Sealed + ToString,
{
    pub fn new(element: E) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::new_tag(&element.to_string())),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(_element: E, node: web_sys::Node) -> Self {
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
                #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
                pub struct [<$tag:camel>];

                impl sealed::Sealed for [<$tag:camel>] {}

                impl fmt::Display for [<$tag:camel>] {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        f.write_str(stringify!($tag))
                    }
                }

                impl<Msg> PartialEq<DynNode<Msg> > for [<$tag:camel>] {
                    fn eq(&self, rhs: &DynNode<Msg> ) -> bool {
                        match rhs.node() {
                            NodeKind::Tag { name, .. } => *name == self.to_string(),
                            _ => false,
                        }
                    }
                }

                impl<Msg> PartialEq<[<$tag:camel>]> for DynNode<Msg>  {
                    fn eq(&self, rhs: &[<$tag:camel>]) -> bool {
                        match self.node() {
                            NodeKind::Tag { name, .. } => *name == rhs.to_string(),
                            _ => false,
                        }
                    }
                }

                pub fn $tag<Msg>() -> HtmlElement<[<$tag:camel>], Msg> {
                    HtmlElement::new([<$tag:camel>])
                }
            )*
        }
    };
}

generate_html_tags![body, div, button, h1, h2, h3];

// =============================================================================
//                          Old version
// =============================================================================

api_planning! {
use crate::{
    BoxNode, ChildrenMut, ChildrenRef, DynNode, IntoNode, Node, NodeTree,
};
use futures::Stream;
use paste::paste;
#[cfg(any(feature = "ssr", feature = "hmr"))]
use serde::{Deserialize, Serialize};
use std::{fmt, marker::PhantomData};

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

#[derive(Educe)]
#[educe(Deref, DerefMut)]
pub struct HtmlElement<E, Msg> {
    element: E,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    #[educe(Target)]
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

    fn children(&self) -> ChildrenRef<Msg> {
        self.node
            .as_ref()
            .expect("attempted to get children after calling `into_node()`")
            .children_rc()
    }

    fn children_mut(&mut self) -> ChildrenMut<Msg> {
        self.node
            .as_mut()
            .expect("attempted to get children after calling `into_node()`")
            .children_mut()
    }

    fn append_child(&mut self, child: DynNode<Msg>) {
        self.node
            .as_mut()
            .expect("attempted to append child after calling `into_node()`")
            .append_child(child);
    }

    fn clear_children(&mut self) {
        self.node
            .as_mut()
            .expect("attempted to append child after calling `into_node()`")
            .clear_children();
    }
}

#[async_trait(?Send)]
impl<E, Msg> IntoNode<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: sealed::Sealed + 'static,
{
    async fn into_node(self) -> DynNode<Msg> {
        self.node
            .expect("called `into_node()` more than once")
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
    async fn into_node(self) -> DynNode<Msg> {
        self.node
            .take()
            .expect("called `into_node()` more than once")
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

                impl<Msg> PartialEq<DynNode<Msg> > for [<$tag:camel>] {
                    fn eq(&self, rhs: &DynNode<Msg> ) -> bool {
                        match rhs.node() {
                            NodeTree::Tag { name, .. } => *name == self.to_string(),
                            _ => false,
                        }
                    }
                }

                impl<Msg> PartialEq<[<$tag:camel>]> for DynNode<Msg>  {
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
}
