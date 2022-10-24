use core::marker::PhantomData;
use futures::{
  channel::mpsc::{UnboundedReceiver, UnboundedSender},
  stream::StreamExt,
};
use std::{
  any::{self, Any, TypeId},
  cell::RefCell,
  collections::HashMap,
  ops::Deref,
  pin::Pin,
  rc::Rc,
};

pub trait DiffableModel {
  type ViewModel;

  fn to_view_model(&self) -> Self::ViewModel;

  fn diff(&self, view_model: &mut Self::ViewModel);
}

impl DiffableModel for () {
  type ViewModel = ();

  fn to_view_model(&self) -> Self::ViewModel {}

  fn diff(&self, _: &mut Self::ViewModel) {}
}

pub trait IntoMsg<Msg>: Sized {
  fn into_msg(self) -> Option<Msg>;
}

impl<Msg> IntoMsg<Msg> for Option<Msg> {
  fn into_msg(self) -> Option<Msg> {
    self
  }
}

impl<Msg> IntoMsg<Msg> for () {
  fn into_msg(self) -> Option<Msg> {
    None
  }
}

#[derive(educe::Educe)]
#[educe(Clone)]
pub struct Ctx<Msg> {
  pub(crate) msg_dispatcher: UnboundedSender<Msg>,
  pub(crate) data: Rc<RefCell<HashMap<any::TypeId, ContextData>>>,
}

impl<Msg> Ctx<Msg> {
  pub(crate) fn new(msg_dispatcher: UnboundedSender<Msg>) -> Self {
    Self {
      msg_dispatcher,
      data: Default::default(),
    }
  }

  #[track_caller]
  pub fn set_context<T: 'static>(
    &self,
    data: T,
  ) -> Result<(), ContextError<T>> {
    use std::collections::hash_map::Entry;

    let type_id = data.type_id();

    match self.data.borrow_mut().entry(type_id) {
      Entry::Occupied(ocupied) => Err(ContextError::AlreadySet {
        data,
        #[cfg(debug_assertions)]
        location: ocupied.get().location,
      }),
      Entry::Vacant(vacant) => {
        vacant.insert(ContextData {
          data: Box::pin(AnyContainer(data)),
          #[cfg(debug_assertions)]
          location: std::panic::Location::caller(),
        });

        Ok(())
      }
    }
  }

  pub fn get_context<T: 'static>(&self) -> Option<&T> {
    let type_id = TypeId::of::<T>();

    if let Some(ContextData { data, .. }) = self.data.borrow().get(&type_id) {
      let data_ptr = data.0.downcast_ref::<T>().unwrap() as *const T;

      // Safety:
      // This is safe because values can be set only once, and they
      // are guaranteed to live for as long as `Ctx` exists.
      // Since `data` is also `Pin`, there's no chance of the `Box<T>`
      // being swapped out, avoiding the possiblity of this becoming
      // a dangling pointer.
      unsafe { Some(&*data_ptr) }
    } else {
      None
    }
  }
}

pub enum ContextError<T = ()> {
  AlreadySet {
    /// The data that was attempted to be set.
    data: T,
    /// The location where the data was originally and successfully set.
    #[cfg(debug_assertions)]
    location: &'static std::panic::Location<'static>,
  },
}

pub(crate) struct ContextData {
  data: Pin<Box<AnyContainer<dyn Any>>>,
  #[cfg(debug_assertions)]
  location: &'static std::panic::Location<'static>,
}

struct AnyContainer<T: ?Sized>(T);

impl<T: ?Sized> Unpin for AnyContainer<T> {}

#[derive(derive_more::Constructor)]
pub struct Runtime<M: DiffableModel, Msg, UF> {
  model: Option<M>,
  view_model: M::ViewModel,
  update_fn: UF,
  msg_receiver: UnboundedReceiver<Msg>,
}

impl<M: DiffableModel, Msg, UF> Runtime<M, Msg, UF>
where
  UF: FnMut(M, Msg) -> M,
{
  pub async fn run(&mut self) -> ! {
    #[cfg(debug_assertions)]
    assert!(self.model.is_some());

    loop {
      if let Some(msg) = self.msg_receiver.next().await {
        let model = self.model.take().unwrap();

        let new_model = (self.update_fn)(model, msg);

        new_model.diff(&mut self.view_model);

        self.model = Some(new_model);
      }
    }
  }
}
