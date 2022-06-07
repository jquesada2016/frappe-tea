mod dyn_child;
mod each;
mod if_;

use crate::NodeTree;
pub use dyn_child::*;
pub use each::*;
pub use if_::*;
use std::marker::PhantomData;

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
    _node: NodeTree<Msg>,
}

pub struct MissingCtx;
pub struct Ctx;
