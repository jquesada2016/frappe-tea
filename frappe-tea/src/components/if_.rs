mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

use futures::{Stream, StreamExt};
use sealed::Sealed;
use wasm_bindgen::UnwrapThrowExt;

use crate::{BoxNode, IntoNode, NodeTree};

pub struct If<T, Msg>
where
    T: Sealed,
{
    state: T,
    node: BoxNode<Msg>,
}

impl<Msg> If<(), Msg>
where
    Msg: 'static,
{
    pub fn new<St, F, const N: usize>(
        bool_stream: St,
        children_fn: F,
    ) -> If<Simple<St, F>, Msg>
    where
        St: Stream<Item = bool>,
        F: Fn() -> [BoxNode<Msg>; N] + 'static,
    {
        If {
            state: Simple {
                bool_stream,
                children_fn,
            },
            node: NodeTree::new_component("If").into_node(),
        }
    }
}

pub struct Simple<St, F> {
    bool_stream: St,
    children_fn: F,
}

impl<St, F> Sealed for Simple<St, F> {}

impl<Msg, St, F, const N: usize> IntoNode<Msg> for If<Simple<St, F>, Msg>
where
    St: Stream<Item = bool>,
    F: Fn() -> [BoxNode<Msg>; N] + 'static,
{
    fn into_node(self) -> BoxNode<Msg> {
        let this = self.node;

        let children_fn = self.state.children_fn;

        let children = this.children();

        self.state.bool_stream.for_each(move |b| {
            let mut lock = children.lock().unwrap_throw();

            if b {
                for child in children_fn() {
                    lock.push(child);
                }
            } else {
                lock.clear();
            }

            async {}
        });

        this
    }
}

impl<Msg, St, F, const N: usize> If<Simple<St, F>, Msg>
where
    St: Stream<Item = bool>,
    F: Fn() -> [BoxNode<Msg>; N] + 'static,
{
    pub fn else_if() {
        todo!()
    }

    pub fn else_() {
        todo!()
    }
}
