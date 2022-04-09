use crate::{ChildrenMut, ChildrenRef, IntoNode, Node, NodeKind};
use futures::lock::Mutex;
use paste::paste;
use sealed::*;
use std::{fmt, future::Future, marker::PhantomData, sync::Arc};

use crate::{Context, DynNode, NodeTree};

mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

// =============================================================================
//                                Traits
// =============================================================================

pub trait Html: Sealed {}

pub trait HtmlFutureExt<Msg>: Html + Future {
    fn text<'a>(&mut self, text: impl ToString) -> &mut Self {
        todo!()
    }

    fn child<'a, Fut>(
        &mut self,
        child_fn: impl FnOnce(Context<Msg>) -> Fut,
    ) -> &mut Self
    where
        Fut: Future<Output = DynNode<Msg>> + Send,
    {
        todo!()
    }
}

// =============================================================================
//                           Structs and Impls
// =============================================================================

pub struct HtmlElement<State, E, Msg> {
    _element: PhantomData<E>,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    node: Option<NodeTree<Msg>>,
    state: State,
    futures: Vec<Box<dyn Future<Output = ()> + Send + Sync>>,
}

impl<E, Msg> Html for HtmlElement<AppliedCtx, E, Msg> where E: Sealed {}

#[async_trait]
impl<E, Msg> Node<Msg> for HtmlElement<AppliedCtx, E, Msg>
where
    E: Send + Sync + 'static,
{
    fn node(&self) -> &NodeKind {
        self.node
            .as_ref()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .node()
    }

    fn node_mut(&mut self) -> &mut NodeKind {
        self.node
            .as_mut()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .node_mut()
    }

    fn cx(&self) -> &Context<Msg> {
        self.node
            .as_ref()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .cx()
    }

    fn set_ctx(&mut self, cx: Context<Msg>) {
        self.node
            .as_mut()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .set_ctx(cx)
    }

    async fn children(&self) -> ChildrenRef<Msg> {
        self.node
            .as_ref()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .children()
            .await
    }

    async fn children_mut(&mut self) -> ChildrenMut<Msg> {
        self.node
            .as_mut()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .children_mut()
            .await
    }

    #[track_caller]
    async fn append_child(&mut self, child: DynNode<Msg>) {
        self.node
            .as_mut()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .append_child(child)
            .await
    }

    async fn clear_children(&mut self) {
        self.node
            .as_mut()
            .expect(
                "attempted to use node interafce after calling `.into_node()`",
            )
            .clear_children()
            .await
    }
}

impl<State, E, Msg> sealed::Sealed for HtmlElement<State, E, Msg> where
    E: sealed::Sealed
{
}

impl<E, Msg> HtmlElement<MissingCtx, E, Msg>
where
    E: sealed::Sealed + ToString,
{
    pub fn new(element: E) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::new_tag(&element.to_string())),
            state: MissingCtx,
            futures: vec![],
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(_element: E, node: web_sys::Node) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::from_raw_node(node)),
            state: MissingCtx,
            futures: vec![],
        }
    }

    pub fn cx(self, cx: Context<Msg>) -> HtmlElement<AppliedCtx, E, Msg> {
        let Self {
            _element,
            mut node,
            futures,
            ..
        } = self;

        node.as_mut().unwrap().set_ctx(cx);

        HtmlElement {
            _element,
            node,
            state: AppliedCtx,
            futures,
        }
    }
}

pub struct MissingCtx;
pub struct AppliedCtx;

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

                pub fn $tag<Msg>() -> HtmlElement<MissingCtx, [<$tag:camel>], Msg> {
                    HtmlElement::new([<$tag:camel>])
                }
            )*
        }
    };
}

generate_html_tags![body, div, button, h1, h2, h3];

api_planning! {
    /// We want an API similar to the following:
    Fragment::new()
        .cx(cx)
        .child(|cx| async move { h1().cx(cx).text("Hello!").await })
        .child(|cx| async move {
            div()
                .cx(cx)
                .child(|cx| async move { p().cx(cx).text("Text").await })
        })

    /// The above could easily be turned into something of the following
    /// form
    view! {
        Fragment [] [
            h1 [] [ "Hello" ],
            div [] [ p [] [ "Text" ] ]

        ]
    }

    /// Or even
    view! {
        <>
            <h1>Hello</h1>
            <div>Text</div>
        </>
    }
}
