use crate::{DynNode, IntoNode};

pub struct DynChild;

impl DynChild {
    pub fn new<Msg>(
        bool_stream: (),
        children_fn: impl FnMut() -> DynNode<Msg>,
    ) -> Self {
        todo!()
    }
}

impl<Msg> IntoNode<Msg> for DynChild {
    fn into_node(self) -> DynNode<Msg> {
        todo!()
    }
}
