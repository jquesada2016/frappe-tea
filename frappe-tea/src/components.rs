// mod dyn_child;
// mod if_;

use crate::{
    ChildrenMut, ChildrenRef, Context, DynNode, Node, NodeKind, NodeTree,
};
// pub use dyn_child::*;
// pub use if_::*;
use std::marker::PhantomData;

pub trait Comp {
    type Props;
}

impl Comp for () {
    type Props = ();
}

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
