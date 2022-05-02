use std::sync::Arc;

use crate::{prelude::Observable, Context, IntoNode, NodeTree};

use super::{Ctx, MissingCtx};

pub struct DynChild<State, Msg, O, F> {
    node: NodeTree<Msg>,
    observer: O,
    child_fn: F,
    _state: State,
}

impl<Msg, O, F, N> DynChild<MissingCtx, Msg, O, F>
where
    O: Observable,
    F: FnMut(&Context<Msg>, O::Item) -> N + 'static,
    N: IntoNode<Msg>,
{
    pub fn new(bool_observer: O, child_fn: F) -> Self {
        let node = NodeTree::new_component("DynNode");

        Self {
            node,
            observer: bool_observer,
            child_fn,
            _state: MissingCtx,
        }
    }

    pub fn cx(self, cx: &Context<Msg>) -> DynChild<Ctx, Msg, O, F> {
        let Self {
            node,
            observer,
            child_fn,
            _state: _,
        } = self;

        node.children.set_cx(cx);

        DynChild {
            node,
            observer,
            child_fn,
            _state: Ctx,
        }
    }
}

impl<Msg, O, F, N> IntoNode<Msg> for DynChild<Ctx, Msg, O, F>
where
    Msg: 'static,
    O: Observable,
    F: FnMut(&Context<Msg>, O::Item) -> N + 'static,
    N: IntoNode<Msg>,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            node,
            observer,
            mut child_fn,
            _state: _,
        } = self;

        let this = node.node.clone();

        let children = node.children.clone();

        let children = Arc::downgrade(&children);

        let cx = node.children.cx().clone();

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
