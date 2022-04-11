mod dyn_child;
// mod if_;

use crate::{
    ChildrenMut, ChildrenRef, Context, DynNode, Node, NodeKind, NodeTree,
};
pub use dyn_child::*;
// pub use if_::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};

// =============================================================================
//                              Traits
// =============================================================================

pub trait Comp {
    type Props;
}

impl Comp for () {
    type Props = ();
}

// =============================================================================
//                          Structs and Impls
// =============================================================================

pub struct Component<C, Msg>
where
    C: Comp,
{
    _component: PhantomData<C>,
    _props: C::Props,
    node: NodeTree<Msg>,
}

impl<C, Msg> Node<Msg> for Component<C, Msg>
where
    C: Comp,
{
    fn cx(&self) -> &Context<Msg> {
        self.node.cx()
    }

    fn set_cx(&mut self, cx: Context<Msg>) {
        self.node.set_cx(cx);
    }

    fn node(&self) -> &NodeKind {
        self.node.node()
    }

    fn node_mut(&mut self) -> &mut NodeKind {
        self.node.node_mut()
    }

    fn children(&self) -> ChildrenRef<Msg> {
        self.node.children()
    }

    fn children_mut(&mut self) -> ChildrenMut<Msg> {
        self.node.children_mut()
    }

    fn append_child(&mut self, child: DynNode<Msg>) {
        self.node.append_child(child);
    }

    fn clear_children(&mut self) {
        self.node.clear_children();
    }
}

/// Helper struct for creating components that need to have shared mutable
/// nodes.
struct RcNode<Msg>(Rc<RefCell<NodeTree<Msg>>>);

impl<Msg> RcNode<Msg> {
    fn new(node_tree: NodeTree<Msg>) -> Self {
        Self(Rc::new(RefCell::new(node_tree)))
    }
}

impl<Msg> Node<Msg> for RcNode<Msg> {
    fn node(&self) -> &NodeKind {
        // self.0.borrow().node()

        todo!()
    }

    fn node_mut(&mut self) -> &mut NodeKind {
        // self.0.borrow_mut().node()

        todo!()
    }

    fn cx(&self) -> &Context<Msg> {
        // self.0.borrow().cx()

        todo!()
    }

    fn set_cx(&mut self, cx: Context<Msg>) {
        self.0.borrow_mut().set_cx(cx);
    }

    fn children(&self) -> ChildrenRef<Msg> {
        // self.0.borrow().children()

        todo!()
    }

    fn children_mut(&mut self) -> ChildrenMut<Msg> {
        // self.0.borrow_mut().children_mut()

        todo!()
    }

    #[track_caller]
    fn append_child(&mut self, child: DynNode<Msg>) {
        self.0.borrow_mut().append_child(child)
    }

    fn clear_children(&mut self) {
        self.0.borrow_mut().clear_children()
    }
}
