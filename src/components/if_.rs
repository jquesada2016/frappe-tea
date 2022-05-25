use crate::{reactive::Observable, Context, IntoNode, NodeTree};

pub struct If<State, Msg> {
    state: State,
    cx: Context<Msg>,
}

impl<Msg> If<(), Msg> {
    pub fn new<O, N: IntoNode<Msg>, F>(
        cx: &Context<Msg>,
        bool_observer: O,
        children_fn: F,
    ) -> If<Simple<O, F>, Msg>
    where
        O: Observable<Item = bool>,
        F: FnMut(O::Item) -> N,
    {
        let cx = Context::from_parent_cx(cx);
        cx.set_dynamic();

        If {
            cx,
            state: Simple {
                observer: bool_observer,
                children_fn,
            },
        }
    }
}

pub struct Simple<O, F> {
    observer: O,
    children_fn: F,
}

impl<Msg, O, F, N> IntoNode<Msg> for If<Simple<O, F>, Msg>
where
    O: Observable<Item = bool>,
    F: FnMut(O::Item) -> N,
    N: IntoNode<Msg>,
{
    fn into_node(self) -> NodeTree<Msg> {
        let this = NodeTree::new_component("If", &self.cx);

        todo!()
    }
}

pub struct ElseIf {}

pub struct Else {}
