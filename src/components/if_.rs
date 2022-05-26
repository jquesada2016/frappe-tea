use std::sync::Arc;

use crate::{reactive::Observable, Context, IntoNode, NodeTree};

pub struct If<State, Msg> {
    state: State,
    cx: Context<Msg>,
}

impl<Msg> If<(), Msg> {
    pub fn new<O, N: IntoNode<Msg>, F>(
        cx: &Context<Msg>,
        bool_observer: O,
        children_fn: F,
    ) -> If<Simple<O, F>, Msg>
    where
        O: Observable<Item = bool>,
        F: FnMut(&Context<Msg>) -> N,
    {
        let cx = Context::from_parent_cx(cx);
        cx.set_dynamic();

        If {
            cx,
            state: Simple {
                observer: bool_observer,
                children_fn,
            },
        }
    }
}

pub struct Simple<O, F> {
    observer: O,
    children_fn: F,
}

impl<Msg, O, F, N> IntoNode<Msg> for If<Simple<O, F>, Msg>
where
    Msg: 'static,
    O: Observable<Item = bool>,
    F: FnMut(&Context<Msg>) -> N + 'static,
    N: IntoNode<Msg>,
{
    fn into_node(self) -> NodeTree<Msg> {
        let mut children_fn = self.state.children_fn;
        let cx = self.cx;

        let this = NodeTree::new_component("If", cx.clone());

        let children = Arc::downgrade(&this.children);
        let node = this.node.clone();

        self.state.observer.subscribe(Box::new(move |b| {
            if let Some(children) = children.upgrade() {
                if b {
                    children.clear();

                    let child = children_fn(&cx).into_node();

                    children.append(&node, child);
                } else {
                    children.clear();
                }
            }
        }));

        this
    }
}

pub struct ElseIf {}

pub struct Else {}
