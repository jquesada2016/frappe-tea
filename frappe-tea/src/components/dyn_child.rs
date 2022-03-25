use futures::Stream;

use crate::{BoxNode, IntoNode};

pub struct DynChild;

impl DynChild {
    pub fn new<Msg, const N: usize>(
        bool_stream: impl Stream<Item = bool>,
        children_fn: impl FnMut() -> [BoxNode<Msg>; N],
    ) -> Self {
        todo!()
    }
}

#[async_trait(?Send)]
impl<Msg> IntoNode<Msg> for DynChild {
    async fn into_node(self) -> BoxNode<Msg> {
        todo!()
    }
}
