use crate::{
    components::DynChild,
    prelude::{is_browser, Observable},
    Context, EventHandler, IntoNode, NodeKind, NodeTree,
};
use paste::paste;
use sealed::*;
use std::{fmt, ops::Deref};

mod sealed {
    pub trait Sealed {}
}

// =============================================================================
//                           Structs and Impls
// =============================================================================

pub struct HtmlElement<E, Msg> {
    _element: E,
    /// This field is `Option<_>` because it gaurds agains calling [IntoNode::into_node`]
    /// more than once.
    node: NodeTree<Msg>,
}

impl<E, Msg> HtmlElement<E, Msg>
where
    Msg: 'static,
    E: Sealed + ToString + 'static,
{
    pub fn new(element: E, cx: &Context<Msg>) -> Self {
        Self {
            node: NodeTree::new_tag(&element.to_string()),
            _element: element,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(element: E, node: web_sys::Node) -> Self {
        Self {
            _element: element,
            node: NodeTree::from_raw_node(node),
        }
    }

    #[track_caller]
    pub fn id(self, id: impl ToString) -> Self {
        self.node.children.cx().id.set_custom_id(id.to_string());

        self
    }

    pub fn text(mut self, text: impl ToString) -> Self {
        let this = &mut self.node;

        let text = text.to_string();

        let text_node = NodeTree::new_text(&text).into_node();

        text_node.children.set_cx(this.children.cx());

        this.append_child(text_node);

        self
    }

    pub fn child<N>(mut self, child_fn: impl FnOnce(&Context<Msg>) -> N) -> Self
    where
        N: IntoNode<Msg>,
    {
        let cx = self.node.children.cx();

        let child = child_fn(cx).into_node();

        self.node.append_child(child);

        self
    }

    pub fn dyn_child<O, N>(
        self,
        bool_observer: O,
        child_fn: impl FnMut(&Context<Msg>, O::Item) -> N + 'static,
    ) -> Self
    where
        O: Observable,
        N: IntoNode<Msg>,
    {
        let this = &self.node;

        let cx = this.children.cx();

        let dyn_child = DynChild::new(bool_observer, child_fn);

        let dyn_child = dyn_child.cx(cx);

        this.children.append(&this.node, dyn_child.into_node());

        self
    }

    #[track_caller]
    pub fn on<F>(mut self, event: impl ToString, mut f: F) -> Self
    where
        F: FnMut(&web_sys::Event) -> Option<Msg> + 'static,
    {
        let this = &mut self.node;

        let msg_dispatcher = this.children.msg_dispatcher();

        match &mut this.node {
            NodeKind::Tag {
                node,
                event_handlers,
                ..
            } => {
                if is_browser() {
                    let handler = gloo::events::EventListener::new(
                        node.as_ref().unwrap().deref(),
                        event.to_string(),
                        move |e| {
                            if let Some(msg_dispatcher) =
                                msg_dispatcher.upgrade()
                            {
                                let msg = f(e);

                                if let Some(msg) = msg {
                                    msg_dispatcher.dispatch_msg(msg);
                                }
                            }
                        },
                    );

                    let location = std::panic::Location::caller();

                    event_handlers.push(EventHandler {
                        _handler: Some(handler),
                        location,
                    })
                }
            }
            _ => unreachable!(),
        }

        self
    }
}

impl<E, Msg> IntoNode<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: Send + Sync + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        self.node.into_node()
    }
}

impl<E, Msg> sealed::Sealed for HtmlElement<E, Msg> where E: sealed::Sealed {}

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

                pub fn $tag<Msg>(cx: &Context<Msg>) -> HtmlElement<[<$tag:camel>], Msg>
                where
                    Msg: 'static
                {
                    HtmlElement::new([<$tag:camel>], cx)
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
