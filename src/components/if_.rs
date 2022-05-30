// TODO: This is a bug, remove it when it get's fixed
#![allow(clippy::suspicious_else_formatting)]

use std::{cell::RefCell, rc::Rc, sync::Arc};

use crate::{prelude::reactive::Observable, Context, IntoNode, NodeTree};

pub struct If<State, Msg> {
    state: State,
    cx: Context<Msg>,
}

impl<Msg> If<(), Msg> {
    pub fn new<O, F, N>(
        cx: &Context<Msg>,
        bool_observer: O,
        child_fn: F,
    ) -> If<Simple<O, F>, Msg>
    where
        O: Observable<Item = bool> + 'static,
        F: FnMut(&Context<Msg>) -> N + 'static,
        N: IntoNode<Msg> + 'static,
    {
        let cx = Context::from_parent_cx(cx);
        cx.set_dynamic();

        If {
            cx,
            state: Simple {
                observer: bool_observer,
                child_fn,
            },
        }
    }
}

pub struct Simple<O, F> {
    observer: O,
    child_fn: F,
}

impl<Msg, O, F, N> IntoNode<Msg> for If<Simple<O, F>, Msg>
where
    Msg: 'static,
    O: Observable<Item = bool>,
    F: FnMut(&Context<Msg>) -> N + 'static,
    N: IntoNode<Msg> + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let mut children_fn = self.state.child_fn;
        let cx = self.cx;

        let this = NodeTree::new_component("If", cx.clone());

        let children = Arc::downgrade(&this.children);
        let node = this.node.clone();

        self.state.observer.subscribe(Box::new(move |b| {
            if let Some(children) = children.upgrade() {
                if b {
                    children.clear();

                    let child = children_fn(&cx).into_node();

                    children.append(&node, child);
                } else {
                    children.clear();
                }
            }
        }));

        this
    }
}

impl<Msg, O, F, N> If<Simple<O, F>, Msg>
where
    O: Observable<Item = bool> + 'static,
    F: FnMut(&Context<Msg>) -> N + 'static,
    N: IntoNode<Msg> + 'static,
{
    pub fn else_if<OO, FF, NN>(
        self,
        bool_observer: OO,
        mut child_fn: FF,
    ) -> If<ElseIf<Msg>, Msg>
    where
        OO: Observable<Item = bool> + 'static,
        FF: FnMut(&Context<Msg>) -> NN + 'static,
        NN: IntoNode<Msg> + 'static,
    {
        let Self {
            cx,
            state:
                Simple {
                    child_fn: mut simple_child_fn,
                    observer,
                },
        } = self;

        If {
            cx,
            state: ElseIf {
                ifs: vec![
                    (
                        Box::new(observer),
                        Box::new(move |cx| simple_child_fn(cx).into_node()),
                    ),
                    (
                        Box::new(bool_observer),
                        Box::new(move |cx| child_fn(cx).into_node()),
                    ),
                ],
            },
        }
    }

    pub fn else_<FF, NN>(self, child_fn: FF) -> If<Else<Msg, FF>, Msg>
    where
        FF: FnMut(&Context<Msg>) -> NN + 'static,
        NN: IntoNode<Msg> + 'static,
    {
        let Self {
            cx,
            state:
                Simple {
                    child_fn: mut simple_child_fn,
                    observer,
                },
        } = self;

        If {
            state: Else {
                ifs: vec![(
                    Box::new(observer),
                    Box::new(move |cx| simple_child_fn(cx).into_node()),
                )],
                child_fn,
            },
            cx,
        }
    }
}

pub struct ElseIf<Msg> {
    #[allow(clippy::type_complexity)]
    ifs: Vec<(
        Box<dyn Observable<Item = bool>>,
        Box<dyn FnMut(&Context<Msg>) -> NodeTree<Msg>>,
    )>,
}

impl<Msg> IntoNode<Msg> for If<ElseIf<Msg>, Msg>
where
    Msg: 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            cx,
            state: ElseIf { ifs },
        } = self;

        let this = NodeTree::new_component("If", cx.clone());

        generate_if_blocks(&this, cx, ifs, None);

        this
    }
}

impl<Msg> If<ElseIf<Msg>, Msg> {
    pub fn else_if<OO, FF, NN>(
        mut self,
        bool_observer: OO,
        mut child_fn: FF,
    ) -> Self
    where
        OO: Observable<Item = bool> + 'static,
        FF: FnMut(&Context<Msg>) -> NN + 'static,
        NN: IntoNode<Msg> + 'static,
    {
        self.state.ifs.push((
            Box::new(bool_observer),
            Box::new(move |cx| child_fn(cx).into_node()),
        ));

        self
    }

    pub fn else_<FF, NN>(self, child_fn: FF) -> If<Else<Msg, FF>, Msg>
    where
        FF: FnMut(&Context<Msg>) -> NN + 'static,
        NN: IntoNode<Msg> + 'static,
    {
        let Self {
            cx,
            state: ElseIf { ifs },
        } = self;

        If {
            state: Else { ifs, child_fn },
            cx,
        }
    }
}

pub struct Else<Msg, F> {
    #[allow(clippy::type_complexity)]
    ifs: Vec<(
        Box<dyn Observable<Item = bool>>,
        Box<dyn FnMut(&Context<Msg>) -> NodeTree<Msg>>,
    )>,
    child_fn: F,
}

impl<Msg, F, N> IntoNode<Msg> for If<Else<Msg, F>, Msg>
where
    Msg: 'static,
    F: FnMut(&Context<Msg>) -> N + 'static,
    N: IntoNode<Msg> + 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        let Self {
            cx,
            state: Else { mut child_fn, ifs },
        } = self;

        let this = NodeTree::new_component("If", cx.clone());

        generate_if_blocks(
            &this,
            cx,
            ifs,
            Some(Box::new(move |cx| child_fn(cx).into_node())),
        );

        this
    }
}

/// Helper function to generate else_if/else blocks.
#[allow(clippy::type_complexity)]
fn generate_if_blocks<Msg>(
    this: &NodeTree<Msg>,
    cx: Context<Msg>,
    ifs: Vec<(
        Box<dyn Observable<Item = bool>>,
        Box<dyn FnMut(&Context<Msg>) -> NodeTree<Msg>>,
    )>,
    else_fn: Option<Box<dyn FnMut(&Context<Msg>) -> NodeTree<Msg>>>,
) where
    Msg: 'static,
{
    let children = Arc::downgrade(&this.children);

    // We need to be able to update and set the current block that should be
    // rendered each time an expr changes
    let exprs = Rc::new(RefCell::new(Vec::with_capacity(ifs.len())));
    let child_fns = Rc::new(RefCell::new(Vec::with_capacity(ifs.len())));
    let last_block_rendered = Rc::new(RefCell::new(Some(usize::MAX)));
    let else_fn = else_fn.map(|else_fn| Arc::new(RefCell::new(else_fn)));

    for (i, (observer, child_fn)) in ifs.into_iter().enumerate() {
        // Save the initial state of all exprs and child_fns
        exprs.borrow_mut().push(false);
        child_fns.borrow_mut().push(child_fn);

        // Subscribe to each individual expr
        observer.subscribe(Box::new(clone!(
            [
                children,
                exprs,
                child_fns,
                cx,
                { this.node } as this_node,
                last_block_rendered
                else_fn,
            ],
            move |b| {
                let mut exprs_borrow = exprs.borrow_mut();

                let mut last_block_rendered_borrow =
                    last_block_rendered.borrow_mut();

                // Update the state of this expr
                exprs_borrow[i] = b;

                // Now, find the first expr that is true
                if let Some((i, _)) =
                    exprs_borrow.iter().enumerate().find(|(_, b)| **b)
                {
                    let last_block =
                        last_block_rendered_borrow.unwrap_or(usize::MAX);

                    if i != last_block {
                        if let Some(children) = children.upgrade() {
                            children.clear();

                            let child = child_fns.borrow_mut()[i](&cx);

                            children.append(&this_node, child);

                            *last_block_rendered_borrow = Some(i);
                        }
                    }
                }
                // If we can't find any, then render the else block
                // and update the last rendered block
                else if last_block_rendered_borrow.is_some() {
                    if let Some(children) = children.upgrade() {
                        children.clear();
                        *last_block_rendered_borrow = None;

                        if let Some(else_block) = &else_fn {
                            let child =
                                else_block.borrow_mut()(&cx);

                            children.append(&this_node, child);
                        }
                    }
                }
            }
        )));
    }
}
