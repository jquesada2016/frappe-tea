use crate::{
  prelude::Ctx,
  utils,
  view::{Comment, Component, IntoView, View, ViewInner, ViewKind},
};
use futures::{Stream, StreamExt};
use wasm_bindgen::JsCast;

pub struct DynChild<Msg, S, F> {
  cx: Ctx<Msg>,
  stream: S,
  view_fn: F,
}

impl<Msg, S: Stream, F, V> DynChild<Msg, S, F>
where
  F: FnMut(Ctx<Msg>, S::Item) -> V,
  V: IntoView<Msg>,
{
  pub fn new(cx: Ctx<Msg>, stream: S, f: F) -> Self {
    Self {
      cx,
      stream,
      view_fn: f,
    }
  }
}

impl<Msg, S, F, V> IntoView<Msg> for DynChild<Msg, S, F>
where
  Msg: 'static,
  S: Stream + 'static,
  F: FnMut(Ctx<Msg>, S::Item) -> V + 'static,
  V: IntoView<Msg>,
{
  fn into_view(self) -> crate::view::View<Msg> {
    let Self {
      cx,
      stream,
      mut view_fn,
    } = self;

    let kind = ViewKind::new_component("DynChild");

    let (opening_node, children) = match &kind {
      ViewKind::Component(Component {
        opening: Comment { node, .. },
        children,
        ..
      }) => (
        node.clone().unchecked_into::<web_sys::Element>(),
        children.clone(),
      ),
      _ => unreachable!(),
    };

    let placeholder = placeholder(cx.clone());

    opening_node
      .after_with_node_1(&placeholder.0.kind.get_node())
      .unwrap();

    children.borrow_mut().push(placeholder);

    let fut = stream.for_each(clone!([cx], move |item| {
      let view = view_fn(cx.clone(), item).into_view();

      let mut children_borrow = children.borrow_mut();

      #[cfg(debug_assertions)]
      assert!(
        !children_borrow.is_empty(),
        "DynChild invarient broken, please file a bug report"
      );

      let child_node = view.0.kind.get_node();

      opening_node.after_with_node_1(&child_node).unwrap();

      *children_borrow = vec![view];

      async {}
    }));

    utils::spawn_local(fut);

    View(ViewInner { cx, kind })
  }
}

fn placeholder<Msg>(cx: Ctx<Msg>) -> View<Msg> {
  View(ViewInner {
    cx,
    kind: ViewKind::new_component("WaitingForInitialRender"),
  })
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
fn append_node_into_component<Msg>(kind: &mut ViewKind<Msg>, child: View<Msg>) {
}
