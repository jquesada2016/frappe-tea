mod dyn_child;
mod each;
mod if_;

use crate::{
    prelude::{Observer, Source},
    Context, NodeTree,
};
pub use dyn_child::*;
pub use each::*;
pub use if_::*;
use std::{cell::RefCell, marker::PhantomData, rc::Rc, sync::Arc};

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

#[derive(Educe)]
#[allow(clippy::type_complexity)]
#[educe(Clone)]
pub struct MsgDispatcher<Msg>(Rc<RefCell<Box<dyn FnMut(Msg)>>>);

impl<Msg> MsgDispatcher<Msg> {
    pub fn dispatch_msg(&self, msg: Msg) {
        self.0.borrow_mut()(msg)
    }
}

pub fn create_local_state<Msg, M, MF, UF>(
    cx: &Context<Msg>,
    initial_model: MF,
    mut update_fn: UF,
) -> (Observer<M>, MsgDispatcher<Msg>)
where
    M: Send + 'static,
    MF: FnOnce() -> M,
    UF: FnMut(&mut M, Msg) + 'static,
{
    let state = cx.local_state.clone();
    let mut lock = state.lock().unwrap();

    let model = Source::new(initial_model());
    let observer = model.observer();
    let model = Box::new(model);

    *lock = Some(model);
    drop(lock);

    let state_weak = Arc::downgrade(&state);

    (
        observer,
        MsgDispatcher(Rc::new(RefCell::new(Box::new(move |msg| {
            if let Some(state) = state_weak.upgrade() {
                let mut lock = state.lock().unwrap();

                let model =
                    lock.as_mut().unwrap().downcast_mut::<Source<M>>().unwrap();

                model.set_with(|m| update_fn(m, msg));
            }
        })))),
    )
}

api_planning! {
    #[component(btn_view)]
    struct Button {
        label: String,
    }

    enum LocalMsg { /* ... */ }

    fn local_update(model: &mut LocalModel, msg: LocalMsg) {

    }

    fn btn_view<Msg>(
        cx: &Context<Msg>,
        props: ButtonProps,
    ) -> impl IntoNode<Msg> {
        let (local_model, local_dispatcher)
            = Button::use_state(
                cx,
                || LocalModel::default(),
                local_update
            );

        button(cx).text(props.label)
    }

    let btn = Button::new(cx).label("Hello!").build();
}
