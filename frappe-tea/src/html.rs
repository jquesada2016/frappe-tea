use crate::{Context, IntoNode, NodeKind, NodeTree};
use paste::paste;
use sealed::*;
use std::{fmt, marker::PhantomData};

mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

// =============================================================================
//                               Constants
// =============================================================================

const USE_AFTER_INTO_NODE: &str =
    "cannot use `HtmlElement` after calling `into_node()`";

// =============================================================================
//                           Structs and Impls
// =============================================================================

pub struct HtmlElement<State, E, Msg> {
    _element: PhantomData<E>,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    node: Option<NodeTree<Msg>>,
    _state: State,
}

impl<E, Msg> HtmlElement<AppliedCtx, E, Msg>
where
    Msg: 'static,
    E: Sealed + 'static,
{
    pub fn text(&mut self, text: impl ToString) -> &mut Self {
        let this = self.node.as_mut().expect(USE_AFTER_INTO_NODE);

        let text = text.to_string();

        let text_node = NodeTree::new_text(&text).into_node();

        text_node.children.set_cx(this.children.cx());

        this.append_child(text_node);

        self
    }

    pub fn child<N>(
        &mut self,
        child_fn: impl FnOnce(&Context<Msg>) -> N,
    ) -> &mut Self
    where
        N: IntoNode<Msg>,
    {
        let cx = self.node.as_ref().expect(USE_AFTER_INTO_NODE).children.cx();

        let child = child_fn(cx).into_node();

        self.node.as_mut().unwrap().append_child(child);

        self
    }
}

impl<E, Msg> IntoNode<Msg> for HtmlElement<AppliedCtx, E, Msg>
where
    Msg: 'static,
    E: Send + Sync + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        self.node
            .expect("called `into_node()` more than once")
            .into_node()
    }
}

impl<E, Msg> IntoNode<Msg> for &mut HtmlElement<AppliedCtx, E, Msg>
where
    Msg: 'static,
    E: Send + Sync + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        self.node
            .take()
            .expect("called `into_node()` more than once")
            .into_node()
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
            _state: MissingCtx,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(_element: E, node: web_sys::Node) -> Self {
        Self {
            _element: PhantomData::default(),
            node: Some(NodeTree::from_raw_node(node)),
            _state: MissingCtx,
        }
    }

    pub fn cx(self, cx: &Context<Msg>) -> HtmlElement<AppliedCtx, E, Msg> {
        let Self {
            _element, mut node, ..
        } = self;

        node.as_mut().unwrap().children.set_cx(cx);

        HtmlElement {
            _element,
            node,
            _state: AppliedCtx,
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

                impl<Msg> PartialEq<NodeTree<Msg> > for [<$tag:camel>] {
                    fn eq(&self, rhs: &NodeTree<Msg> ) -> bool {
                        match &rhs.node {
                            NodeKind::Tag { name, .. } => *name == self.to_string(),
                            _ => false,
                        }
                    }
                }

                impl<Msg> PartialEq<[<$tag:camel>]> for NodeTree<Msg>  {
                    fn eq(&self, rhs: &[<$tag:camel>]) -> bool {
                        match &self.node {
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
