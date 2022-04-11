use crate::{prelude::Observable, DynNode, IntoNode, NodeTree};

pub struct DynChild<Msg, O> {
    node: NodeTree<Msg>,
    observer: O,
}

impl<Msg, O> DynChild<Msg, O>
where
    O: Observable<Item = bool>,
{
    pub fn new(
        bool_observer: O,
        children_fn: impl FnMut() -> DynNode<Msg>,
    ) -> Self {
        let node = NodeTree::new_component("DynNode");

        Self {
            node,
            observer: bool_observer,
        }
    }
}

impl<Msg, O> IntoNode<Msg> for DynChild<Msg, O>
where
    O: Observable<Item = bool>,
{
    fn into_node(self) -> DynNode<Msg> {
        let Self { node, observer } = self;

        observer.subscribe(Box::new(|v| {
            // if v {
            //     // render
            // } else {
            //     // Remove
            // }
        }));

        todo!()
    }
}
