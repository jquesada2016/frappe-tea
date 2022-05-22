use crate::{
    components::DynChild, prelude::Observable, utils::is_browser, Context,
    EventHandler, IntoNode, NodeKind, NodeTree,
};
use paste::paste;
use sealed::*;
use std::{collections::HashMap, fmt};

mod sealed {
    pub trait Sealed {}
}

// =============================================================================
//                           Structs and Impls
// =============================================================================

pub struct HtmlElement<E, Msg> {
    element: E,
    cx: Context<Msg>,
    children: Vec<NodeTree<Msg>>,
    #[allow(clippy::type_complexity)]
    event_listeners: HashMap<
        String,
        Vec<(
            &'static std::panic::Location<'static>,
            Box<dyn FnMut(&web_sys::Event) -> Option<Msg>>,
        )>,
    >,
}

impl<E, Msg> HtmlElement<E, Msg>
where
    Msg: 'static,
    E: Sealed + ToString + 'static,
{
    pub fn new(element: E, cx: &Context<Msg>) -> Self {
        Self {
            cx: Context::from_parent_cx(cx),
            element,
            children: vec![],
            event_listeners: HashMap::default(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(
        _cx: &Context<Msg>,
        _element: E,
        _node: web_sys::Node,
    ) -> Self {
        todo!()
    }

    #[track_caller]
    pub fn id(self, id: impl ToString) -> Self {
        self.cx.id.set_custom_id(id.to_string());

        self
    }

    pub fn text(mut self, text: impl ToString) -> Self {
        let text = text.to_string();

        let text_node = NodeTree::new_text(&text).into_node();

        text_node.children.set_cx(&self.cx);

        self.children.push(text_node);

        self
    }

    pub fn child<N>(mut self, child_fn: impl FnOnce(&Context<Msg>) -> N) -> Self
    where
        N: IntoNode<Msg>,
    {
        let child = child_fn(&self.cx).into_node();

        self.children.push(child);

        self
    }

    pub fn dyn_child<O, N>(
        mut self,
        bool_observer: O,
        child_fn: impl FnMut(&Context<Msg>, O::Item) -> N + 'static,
    ) -> Self
    where
        O: Observable,
        N: IntoNode<Msg>,
    {
        let dyn_child = DynChild::new(bool_observer, child_fn);

        let dyn_child = dyn_child.cx(&self.cx).into_node();

        self.children.push(dyn_child);

        self
    }

    #[track_caller]
    pub fn on<F>(mut self, event: impl ToString, f: F) -> Self
    where
        F: FnMut(&web_sys::Event) -> Option<Msg> + 'static,
    {
        // Needing an event handler automatically makes this node dynamic, since
        // we need to be able to attach the event listener
        self.cx.set_dynamic();

        let location = std::panic::Location::caller();

        #[cfg(target_arch = "wasm32")]
        let handler = {
            if is_browser() {
                Some(Box::new(f))
            } else {
                None
            }
        };

        if let Some(handler) = handler {
            self.event_listeners
                .entry(event.to_string())
                .or_default()
                .push((location, handler));
        }

        self
    }
}

impl<E, Msg> IntoNode<Msg> for HtmlElement<E, Msg>
where
    Msg: 'static,
    E: ToString + Send + Sync + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let mut this = NodeTree::new_tag(&self.element.to_string(), &self.cx);

        for child in self.children {
            this.append_child(child);
        }

        let mut event_handlers = Vec::with_capacity(self.event_listeners.len());

        #[cfg(target_arch = "wasm32")]
        if let Some(node) = this.node.node() {
            let msg_dispatcher = self.cx.msg_dispatcher();

            for (event, handlers) in self.event_listeners {
                for (location, mut f) in handlers {
                    let handler = gloo::events::EventListener::new(
                        node,
                        event.clone(),
                        clone!([msg_dispatcher], move |e| {
                            if let Some(msg_dispatcher) =
                                msg_dispatcher.upgrade()
                            {
                                if let Some(msg) = f(e) {
                                    msg_dispatcher.dispatch_msg(msg);
                                }
                            }
                        }),
                    );

                    let handler = EventHandler {
                        location,
                        _handler: Some(handler),
                    };

                    event_handlers.push(handler);
                }
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        for (event, handlers) in self.event_listeners {
            for (location, _) in handlers {
                event_handlers.push(EventHandler { location });
            }
        }

        match &mut this.node {
            NodeKind::Tag {
                event_handlers: eh, ..
            } => {
                *eh = event_handlers;
            }
            _ => unreachable!(),
        }

        this
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
