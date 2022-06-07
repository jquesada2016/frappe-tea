use std::sync::Arc;

use crate::{prelude::Observable, Context, IntoNode, NodeTree};

pub struct DynChild<Msg, O, F> {
    cx: Context<Msg>,
    observer: O,
    child_fn: F,
}

impl<Msg, O, F, N> DynChild<Msg, O, F>
where
    O: Observable,
    F: FnMut(&Context<Msg>, &O::Item) -> N + 'static,
    N: IntoNode<Msg>,
{
    pub fn new(cx: &Context<Msg>, bool_observer: O, child_fn: F) -> Self {
        Self {
            cx: Context::from_parent_cx(cx),
            observer: bool_observer,
            child_fn,
        }
    }
}

impl<Msg, O, F, N> IntoNode<Msg> for DynChild<Msg, O, F>
where
    Msg: 'static,
    O: Observable,
    F: FnMut(&Context<Msg>, &O::Item) -> N + 'static,
    N: IntoNode<Msg>,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            cx,
            observer,
            mut child_fn,
        } = self;

        let node = NodeTree::new_component("DynChild", cx);

        let this = node.node.clone();

        let children = node.children.clone();

        let children = Arc::downgrade(&children);

        let cx = node.children.cx.clone();

        observer.subscribe(Box::new(move |v| {
            if let Some(children) = children.upgrade() {
                // Remove the existing children
                children.clear();

                // Add the new children
                let child = child_fn(&cx, v).into_node();

                children.append(&this, child);
            }
        }));

        node
    }
}
