mod sealed {
    pub trait Sealed {}

    impl Sealed for () {}
}

use futures::{Stream, StreamExt};
use sealed::Sealed;
use std::{cell::RefCell, future::Future, rc::Rc};
use wasm_bindgen::UnwrapThrowExt;

use crate::{prelude::execute_async, BoxNode, IntoNode, NodeTree};

pub struct If<T>
where
    T: Sealed,
{
    state: T,
}

impl If<()> {
    pub fn new<Msg, St, F, Fut, const N: usize>(
        bool_stream: St,
        children_fn: F,
    ) -> If<Simple<St, F>>
    where
        St: Stream<Item = bool>,
        F: FnMut() -> Fut + 'static,
        Fut: Future<Output = [BoxNode<Msg>; N]>,
    {
        If {
            state: Simple {
                bool_stream,
                children_fn,
            },
        }
    }
}

pub struct Simple<St, F> {
    bool_stream: St,
    children_fn: F,
}

impl<St, F> Sealed for Simple<St, F> {}

#[async_trait(?Send)]
impl<Msg, St, F, Fut, const N: usize> IntoNode<Msg> for If<Simple<St, F>>
where
    Msg: 'static,
    St: Stream<Item = bool> + 'static,
    F: FnMut() -> Fut + 'static,
    Fut: Future<Output = [BoxNode<Msg>; N]>,
{
    async fn into_node(self) -> BoxNode<Msg> {
        let node = NodeTree::new_component("If").into_node().await;

        let mut children_fn = self.state.children_fn;

        let children = node.children_rc();

        let mut bool_stream = self.state.bool_stream.boxed_local();

        // We need to get the first item from the stream to guerentee
        // synchronousity
        let b = bool_stream.next().await;

        if let Some(b) = b {
            if b {
                for child in children_fn().await {
                    children.borrow_mut().push(child);
                }
            } else {
                /* do nothing */
            }
        }

        let children_fn = Rc::new(RefCell::new(children_fn));

        // We need to move this on to another async execution context
        // because if we were to await `.for_each(/* ... */)` here, we
        // would be stuck here untill the stream ended, which is
        // the opposite of what we want...
        execute_async(bool_stream.for_each(move |b| {
            cloned![[children, children_fn], async move {
                let mut children_borrow = children.borrow_mut();

                if b {
                    for child in children_fn.borrow_mut()().await {
                        children_borrow.push(child);
                    }
                } else {
                    children_borrow.clear();
                }
            }]
        }));

        node
    }
}

impl<Msg, St, F, const N: usize> If<Simple<St, F>>
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
