use crate::{reactive::Observable, Context, IntoNode, NodeTree};
use std::sync::Arc;

pub struct Each<Kind, Msg> {
    cx: Context<Msg>,
    kind: Kind,
}

pub struct Iter<O, F> {
    observer: O,
    each_fn: F,
}

impl<Msg> Each<(), Msg> {
    pub fn iter<I, N, F, O>(
        cx: &Context<Msg>,
        observer_iter: O,
        each_fn: F,
    ) -> Each<Iter<O, F>, Msg>
    where
        O: Observable<Item = I> + 'static,
        for<'a> &'a I: IntoIterator,
        F: FnMut(&Context<Msg>, <&I as IntoIterator>::Item) -> N + 'static,
        N: IntoNode<Msg> + 'static,
    {
        Each {
            cx: Context::from_parent_cx(cx),
            kind: Iter {
                observer: observer_iter,
                each_fn,
            },
        }
    }
}

impl<Msg, I, N, F, O> IntoNode<Msg> for Each<Iter<O, F>, Msg>
where
    Msg: 'static,
    O: Observable<Item = I>,
    for<'a> &'a I: IntoIterator,
    F: FnMut(&Context<Msg>, <&I as IntoIterator>::Item) -> N + 'static,
    N: IntoNode<Msg> + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            cx,
            kind:
                Iter {
                    observer,
                    mut each_fn,
                },
        } = self;

        let this = NodeTree::new_component("Each", cx.clone());
        let this_node = this.node.clone();
        let children = Arc::downgrade(&this.children);

        observer.subscribe(Box::new(move |items| {
            if let Some(children) = children.upgrade() {
                children.clear();

                for item in items {
                    let child = each_fn(&cx, item).into_node();

                    children.append(&this_node, child);
                }
            }
        }));

        this
    }
}
