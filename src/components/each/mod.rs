// use std::sync::Arc;

// use crate::{reactive::Observable, Context, IntoNode, NodeTree};

// pub struct Each<Kind, Msg> {
//     cx: Context<Msg>,
//     kind: Kind,
// }

// pub struct Iter<O, F> {
//     observer: O,
//     each_fn: F,
// }

// impl<Msg> Each<(), Msg> {
//     pub fn iter<I, N, F, O>(
//         cx: &Context<Msg>,
//         observer_iter: O,
//         each_fn: F,
//     ) -> Each<Iter<O, F>, Msg>
//     where
//         O: Observable<Item = I> + 'static,
//         I: IntoIterator,
//         F: FnMut(&Context<Msg>, I::Item) -> N + 'static,
//         N: IntoNode<Msg> + 'static,
//     {
//         Each {
//             cx: Context::from_parent_cx(cx),
//             kind: Iter {
//                 observer: observer_iter,
//                 each_fn,
//             },
//         }
//     }
// }
