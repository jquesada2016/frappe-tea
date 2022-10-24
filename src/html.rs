use crate::{
  runtime::{Ctx, IntoMsg},
  view::{IntoView, View, ViewInner, ViewKind},
};
use futures::SinkExt;
use std::collections::{HashMap, HashSet};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::JsValue;

pub trait HtmlElementMetadata {
  /// The name of the element, such as `a`, `p`, `div`, etc.
  fn name(&self) -> String;

  /// Indicates if the element is void, or self-closing, such
  /// as `<input>` or `<br>`.
  fn is_void(&self) -> bool {
    false
  }
}

pub struct HtmlElement<El, Msg = ()> {
  cx: Ctx<Msg>,
  kind: El,
  attributes: HashMap<String, String>,
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  props: HashMap<String, JsValue>,
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  event_listeners:
    Vec<(String, Box<dyn FnMut(&web_sys::Event) -> Option<Msg>>)>,
  children: Vec<Box<dyn FnOnce(Ctx<Msg>) -> View<Msg>>>,
}

impl<El: HtmlElementMetadata, Msg: 'static> IntoView<Msg>
  for HtmlElement<El, Msg>
{
  fn into_view(self) -> View<Msg> {
    let Self {
      cx,
      kind,
      attributes,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      props,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      event_listeners,
      children,
    } = self;

    let mut kind = if kind.is_void() {
      ViewKind::new_void_html(
        &kind.name(),
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        props,
      )
    } else {
      ViewKind::new_html(
        &kind.name(),
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        props,
      )
    };

    kind.set_attributes(attributes);

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    kind.set_event_listeners(|n| {
      event_listeners
        .into_iter()
        .map(|(e, mut handler)| {
          let dispatcher = cx.msg_dispatcher.clone();

          gloo::events::EventListener::new(n, e, move |e| {
            let res = handler(e);

            if let Some(msg) = res {
              wasm_bindgen_futures::spawn_local(clone!(
                [mut dispatcher],
                async move {
                  let _ = dispatcher.send(msg).await;
                }
              ));
            }
          })
        })
        .collect()
    });

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let parent_node = kind.get_node();

    let children = children
      .into_iter()
      .map(|f| f(cx.clone()))
      .map(|mut child_view| {
        #[cfg(all(target_arch = "wasm32", feature = "web"))]
        {
          let child_node = child_view.0.kind.get_node();

          parent_node.append_child(&child_node).unwrap();

          child_view.0.parent = Some(parent_node.clone());
        }

        child_view
      })
      .collect::<Vec<_>>();

    kind.set_children(children);

    View(ViewInner {
      cx,
      kind,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      parent: None,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      prev_sibling: None,
    })
  }
}

impl<El: HtmlElementMetadata, Msg: 'static> HtmlElement<El, Msg> {
  pub fn new(cx: Ctx<Msg>, kind: El) -> Self {
    Self {
      cx,
      kind,
      attributes: Default::default(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      props: Default::default(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      event_listeners: Default::default(),
      children: Default::default(),
    }
  }

  pub fn attr(mut self, name: impl ToString, value: impl ToString) -> Self {
    #[cfg(debug_assertions)]
    {
      match name.to_string().as_str() {
        "id" => panic!("`id` attribute should be set via `HtmlElement::id()`"),
        "class" => panic!("`class` should be set through `HtmlElement::class`"),
        "style" => panic!("`style` should be set through `HtmlElement::style`"),
        _ => {}
      }
    }

    self.attributes.insert(name.to_string(), value.to_string());

    self
  }

  pub fn class(mut self, name: impl ToString) -> Self {
    let classes = self.attributes.entry("class".to_string()).or_default();

    let mut class_set =
      HashSet::<_, std::collections::hash_map::RandomState>::from_iter(
        classes.split_ascii_whitespace().map(ToString::to_string),
      );

    class_set.insert(name.to_string());

    *classes = class_set
      .into_iter()
      .intersperse(" ".to_string())
      .collect::<String>();

    self
  }

  pub fn text(mut self, text: impl ToString) -> Self {
    let text = self::text(self.cx.clone(), text);

    self.children.push(Box::new(|_| text));

    self
  }

  pub fn child<V: IntoView<Msg>>(
    mut self,
    f: impl FnOnce(Ctx<Msg>) -> V + 'static,
  ) -> Self {
    let child_fn = Box::new(|cx| f(cx).into_view());

    self.children.push(child_fn);

    self
  }

  pub fn on<F, IMsg>(mut self, event: impl ToString, mut handler: F) -> Self
  where
    F: FnMut(&web_sys::Event) -> IMsg + 'static,
    IMsg: IntoMsg<Msg> + 'static,
  {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    self
      .event_listeners
      .push((event.to_string(), Box::new(move |e| handler(e).into_msg())));

    self
  }
}

#[derive(derive_more::Display)]
#[non_exhaustive]
pub enum AnyElement {
  #[display(fmt = "div")]
  Div,
  #[display(fmt = "p")]
  P,
  #[display(fmt = "button")]
  Button,
  #[display(fmt = "input")]
  Input,
}

impl HtmlElementMetadata for AnyElement {
  fn name(&self) -> String {
    self.to_string()
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl From<AnyElement> for web_sys::Node {
  fn from(value: AnyElement) -> Self {
    gloo::utils::document()
      .create_element(&value.to_string())
      .expect("element to be created")
      .into()
  }
}

impl AnyElement {
  pub fn is_void(&self) -> bool {
    matches!(self, Self::Input)
  }
}

pub fn div<Msg: 'static>(cx: Ctx<Msg>) -> HtmlElement<AnyElement, Msg> {
  HtmlElement::new(cx, AnyElement::Div)
}

pub fn p<Msg: 'static>(cx: Ctx<Msg>) -> HtmlElement<AnyElement, Msg> {
  HtmlElement::new(cx, AnyElement::P)
}

pub fn button<Msg: 'static>(cx: Ctx<Msg>) -> HtmlElement<AnyElement, Msg> {
  HtmlElement::new(cx, AnyElement::Button)
}

pub fn text<Msg>(cx: Ctx<Msg>, text: impl ToString) -> View<Msg> {
  View(ViewInner {
    cx,
    kind: ViewKind::new_text(&text.to_string()),
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    parent: None,
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    prev_sibling: None,
  })
}
