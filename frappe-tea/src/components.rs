use crate::{BoxNode, IntoNode, Node, NodeTree};
use std::{
    cell::{Ref, RefMut},
    marker::PhantomData,
};

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
    props: C::Props,
    node: NodeTree<Msg>,
}

impl<C, Msg> Node<Msg> for Component<C, Msg>
where
    C: Comp,
{
    fn node(&self) -> &NodeTree<Msg> {
        &self.node
    }

    fn children(&self) -> Ref<Vec<BoxNode<Msg>>> {
        self.node.children()
    }

    fn children_mut(&mut self) -> RefMut<Vec<BoxNode<Msg>>> {
        self.node.children_mut()
    }

    fn append_child(&mut self, child: BoxNode<Msg>) {
        self.node.append_child(child);
    }
}
