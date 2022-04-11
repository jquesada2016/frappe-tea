use std::{cell::RefCell, rc::Rc};

use crate::{prelude::Observable, IntoNode, NodeTree};

pub struct DynChild<Msg, O, F> {
    node: Rc<RefCell<NodeTree<Msg>>>,
    observer: O,
    child_fn: F,
}

impl<Msg, O, F> DynChild<Msg, O, F>
where
    O: Observable<Item = bool>,
    F: FnMut() -> NodeTree<Msg> + 'static,
{
    pub fn new(bool_observer: O, children_fn: F) -> Self {
        let node = Rc::new(RefCell::new(NodeTree::new_component("DynNode")));

        Self {
            node,
            observer: bool_observer,
            child_fn: children_fn,
        }
    }
}

impl<Msg, O, F> IntoNode<Msg> for DynChild<Msg, O, F>
where
    Msg: 'static,
    O: Observable<Item = bool>,
    F: FnMut() -> NodeTree<Msg> + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            node,
            observer,
            mut child_fn,
        } = self;

        observer.subscribe(Box::new(move |v| {
            if v {
                let child = child_fn().into_node();

                node.borrow_mut().append_child(child);
            } else {
                node.borrow_mut().clear_children();
            }
        }));

        todo!()
    }
}
