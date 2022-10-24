#![feature(iter_intersperse, min_specialization)]

#[macro_use]
extern crate clone_macro;

#[macro_use]
mod utils;
mod html;
mod runtime;
mod view;

pub mod prelude {
  use super::*;

  pub use super::App;
  pub use html::*;
  pub use runtime::Ctx;
  pub use view::View;
}

use runtime::DiffableModel;
use view::IntoView;

/// Represents an app.
pub struct App<M: DiffableModel, Msg, UF> {
  rt: runtime::Runtime<M, Msg, UF>,
  view: view::View<Msg>,
}

impl<M, Msg, UF> App<M, Msg, UF>
where
  M: DiffableModel,
  UF: FnMut(M, Msg) -> M,
{
  pub fn new<V: IntoView<Msg>>(
    init_model: impl FnOnce() -> M,
    update_fn: UF,
    view_fn: impl FnOnce(&M::ViewModel, runtime::Ctx<Msg>) -> V,
  ) -> Self {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    let model = init_model();
    let view_model = model.to_view_model();

    let cx = runtime::Ctx::new(tx);

    let view = view_fn(&view_model, cx).into_view();

    let rt = runtime::Runtime::new(Some(model), view_model, update_fn, rx);

    Self { rt, view }
  }

  /// Runs the app.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub async fn run(&mut self, mount_target_node: &web_sys::Node) -> ! {
    let root_node = self.view.0.kind.get_node();

    mount_target_node
      .append_child(&root_node)
      .expect("mounting to succeed");

    self.rt.run().await
  }

  /// Renders the app to a [`String`].
  #[cfg(feature = "ssr")]
  pub fn render_to_string(&self) -> String {
    self.view.to_string()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use prelude::*;

  #[test]
  fn hello_world() {
    enum Msg {
      Increment,
      Decrement,
    }

    let app = App::new(
      || (),
      |_, _| (),
      |_, cx: Ctx<Msg>| {
        div(cx)
          .child(|cx| {
            p(cx).attr("attr", "val").class("counter-class").text("0")
          })
          .child(|cx| {
            div(cx)
              .child(|cx| {
                button(cx).text("-").on("click", |_| Some(Msg::Decrement))
              })
              .child(|cx| {
                button(cx).text("+").on("click", |_| Some(Msg::Increment))
              })
          })
      },
    );

    assert_eq!(
      app.render_to_string(),
      "<div><p attr=\"val\" \
       class=\"counter-class\">0</p><div><button>-</button><button>+</\
       button></div></div>"
    )
  }
}
