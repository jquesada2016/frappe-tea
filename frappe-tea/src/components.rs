mod dyn_child;
mod if_;

use crate::{BoxNode, IntoNode, Node, NodeTree};
pub use dyn_child::*;
pub use if_::*;
use std::{
    cell::{Ref, RefCell, RefMut},
    marker::PhantomData,
    ops,
    rc::Rc,
    sync::{Arc, Mutex},
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

    #[cfg(not(target_arch = "wasm32"))]
    fn children_rc(&self) -> Arc<Mutex<Vec<BoxNode<Msg>>>> {
        self.node.children_rc()
    }

    #[cfg(target_arch = "wasm32")]
    fn children_rc(&self) -> Rc<RefCell<Vec<BoxNode<Msg>>>> {
        self.node.children_rc()
    }

    fn children<'a>(
        &'a self,
    ) -> Box<dyn ops::Deref<Target = Vec<BoxNode<Msg>>> + 'a> {
        self.node.children()
    }

    fn children_mut(
        &mut self,
    ) -> Box<dyn ops::DerefMut<Target = Vec<BoxNode<Msg>>>> {
        self.node.children_mut()
    }

    fn append_child(&mut self, child: BoxNode<Msg>) {
        self.node.append_child(child);
    }
}
