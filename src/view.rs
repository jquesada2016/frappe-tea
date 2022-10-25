use crate::runtime::Ctx;
use error_stack::{report, Context};
use std::{cell::RefCell, collections::HashMap, fmt, ops::Deref, rc::Rc};
#[cfg(all(target_arch = "wasm32", feature = "web"))]
use wasm_bindgen::{JsCast, JsValue};

pub trait IntoView<Msg> {
  fn into_view(self) -> View<Msg>;
}

#[derive(Clone, Copy, Debug, derive_more::Display)]
enum Error {
  #[display(fmt = "element name is invalid")]
  InvalidElementName,
}

impl Context for Error {}

#[derive(derive_more::Display)]
pub struct View<Msg>(pub(crate) ViewInner<Msg>);

impl<Msg> IntoView<Msg> for View<Msg> {
  fn into_view(self) -> View<Msg> {
    self
  }
}

/// The non-public struct for [`View`].
#[derive(derive_more::Display)]
#[display(fmt = "{kind}")]
pub(crate) struct ViewInner<Msg> {
  /// The runtime context.
  pub cx: Ctx<Msg>,
  /// The kind of [`View`].
  pub kind: ViewKind<Msg>,
}

/// The kind of [`View`].
#[derive(derive_more::Display)]
pub(crate) enum ViewKind<Msg> {
  Html(Html<Msg>),
  VoidHtml(VoidHtml),
  Text(Text),
  Comment(Comment),
  Component(Component<Msg>),
}

impl<Msg> ViewKind<Msg> {
  pub fn new_html(
    name: &str,
    #[cfg(all(target_arch = "wasm32", feature = "web"))] props: HashMap<
      String,
      JsValue,
    >,
  ) -> Self {
    Self::Html(Html::new(
      name,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      props,
    ))
  }

  pub fn new_void_html(
    name: &str,
    #[cfg(all(target_arch = "wasm32", feature = "web"))] props: HashMap<
      String,
      JsValue,
    >,
  ) -> Self {
    Self::VoidHtml(VoidHtml::new(
      name,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      props,
    ))
  }

  pub fn new_text(text: &str) -> Self {
    Self::Text(Text::new(text))
  }

  pub fn new_comment(content: &str) -> Self {
    Self::Comment(Comment::new(content))
  }

  pub fn new_component(name: &str) -> Self {
    Self::Component(Component::new(name))
  }

  /// Gets the backing [`Node`].
  ///
  /// [Node]: web_sys::Node
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub fn get_node(&self) -> web_sys::Node {
    match self {
      Self::Html(Html { node, .. }) => node.clone(),
      Self::VoidHtml(VoidHtml { node, .. }) => node.clone(),
      Self::Text(Text { node, .. }) => node.clone(),
      Self::Comment(Comment { node, .. }) => node.clone(),
      Self::Component(Component { fragment, .. }) => fragment.clone().into(),
    }
  }

  /// Sets the children for [`Html`] and [`Component`] views,
  /// does nothing on others.
  pub fn set_children(&mut self, new_children: Vec<View<Msg>>) {
    if let Self::Html(Html { children, .. }) = self {
      *children = new_children;
    }
  }

  pub fn set_attributes(&mut self, attrs: HashMap<String, String>) {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      let node = self.get_node();

      attrs.iter().for_each(|(n, v)| {
        node
          .unchecked_ref::<web_sys::Element>()
          .set_attribute(n, v)
          .expect("attribute to be valid");
      });
    }

    match self {
      Self::Html(Html { attributes, .. })
      | Self::VoidHtml(VoidHtml { attributes, .. }) => *attributes = attrs,
      _ => {}
    }
  }

  /// Sets the event listeners for [`Html`] and [`VoidHtml`]
  /// views, does nothing on others.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub fn set_event_listeners(
    &mut self,
    f: impl FnOnce(&web_sys::Node) -> Vec<gloo::events::EventListener>,
  ) {
    match self {
      Self::Html(Html {
        node,
        event_listeners,
        ..
      })
      | Self::VoidHtml(VoidHtml {
        node,
        event_listeners,
        ..
      }) => *event_listeners = f(node),
      _ => {}
    }
  }
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl<Msg> Drop for ViewKind<Msg> {
  fn drop(&mut self) {
    match self {
      Self::Html(Html { node, .. })
      | Self::VoidHtml(VoidHtml { node, .. })
      | Self::Text(Text { node, .. }) => {
        node.unchecked_ref::<web_sys::Element>().remove();
      }
      // No need to remove it from the DOM, as this will happen automatically
      // when its' containing `Comment`s are dropped
      Self::Component(_) | Self::Comment(Comment { .. }) => {}
    }
  }
}

/// Represents and HTML element.
pub(crate) struct Html<Msg> {
  /// Name of the HTML element, such as `div` or `a`.
  name: String,
  /// The reference to the [`Node`].
  ///
  /// [Node]: web_sys::Node
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  /// List of HTML attributes, such as `class` and `id`.
  attributes: HashMap<String, String>,
  /// List of props, such as `value` and `checked`.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  _props: HashMap<String, JsValue>,
  /// List of event listeners.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  event_listeners: Vec<gloo::events::EventListener>,
  /// List of children to this [`View`].
  children: Vec<View<Msg>>,
}

impl<Msg> fmt::Display for Html<Msg> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let Self {
      name,
      attributes,
      children,
      ..
    } = self;

    f.write_fmt(format_args!("<{name}"))?;

    if !attributes.is_empty() {
      for (key, value) in attributes {
        f.write_fmt(format_args!(r#" {key}="{value}""#))?;
      }
    }

    if !children.is_empty() {
      f.write_str(">")?;

      for child in children.deref() {
        child.fmt(f)?;
      }

      f.write_fmt(format_args!("</{name}>"))
    } else {
      f.write_str(" />")
    }
  }
}

impl<Msg> Html<Msg> {
  pub fn new(
    name: &str,
    #[cfg(all(target_arch = "wasm32", feature = "web"))] props: HashMap<
      String,
      JsValue,
    >,
  ) -> Self {
    #[cfg(debug_assertions)]
    assert_tag_name_is_valid(name);

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = gloo::utils::document()
      .create_element(name)
      .map_err(|err| {
        report!(Error::InvalidElementName).attach_printable(format!("{err:#?}"))
      })
      .expect("tag name to be valid")
      .unchecked_into();

    Self {
      name: name.to_owned(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
      attributes: Default::default(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      _props: props,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      event_listeners: Default::default(),
      children: Default::default(),
    }
  }
}

pub(crate) struct VoidHtml {
  /// Name of the HTML element, such as `div` or `a`.
  name: String,
  /// The reference to the [`Node`].
  ///
  /// [Node]: web_sys::Node
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
  /// List of HTML attributes, such as `class` and `id`.
  attributes: HashMap<String, String>,
  /// List of props, such as `value` and `checked`.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  _props: HashMap<String, JsValue>,
  /// List of event listeners.
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  event_listeners: Vec<gloo::events::EventListener>,
}

impl fmt::Display for VoidHtml {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let Self {
      name, attributes, ..
    } = self;

    {
      f.write_fmt(format_args!("<{name}"))?;

      if !attributes.is_empty() {
        for (key, value) in attributes {
          f.write_fmt(format_args!(r#" {key}="{value}""#))?;
        }
      }

      f.write_str(">")
    }
  }
}

impl VoidHtml {
  pub fn new(
    name: &str,
    #[cfg(all(target_arch = "wasm32", feature = "web"))] props: HashMap<
      String,
      JsValue,
    >,
  ) -> Self {
    #[cfg(debug_assertions)]
    assert_tag_name_is_valid(name);

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = gloo::utils::document()
      .create_element(name)
      .map_err(|err| {
        report!(Error::InvalidElementName).attach_printable(format!("{err:#?}"))
      })
      .expect("tag name to be valid")
      .unchecked_into();

    Self {
      name: name.to_owned(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
      attributes: Default::default(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      _props: props,
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      event_listeners: Default::default(),
    }
  }
}

/// Represents a test node.
#[derive(derive_more::Display)]
#[display(fmt = "{text}")]
pub(crate) struct Text {
  /// The text content.
  text: String,
  /// Reference to the [`Node`]
  ///
  /// [Node]: web_sys::Node
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  node: web_sys::Node,
}

impl Text {
  pub fn new(text: &str) -> Self {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = gloo::utils::document()
      .create_text_node(text)
      .unchecked_into::<web_sys::Node>();

    Self {
      text: text.to_owned(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
    }
  }
}

/// Represents a comment node.
#[derive(derive_more::Display)]
#[display(fmt = "<!-- {content} -->")]
pub(crate) struct Comment {
  /// The text content.
  content: String,
  /// Reference to the [`Node`]
  ///
  /// [Node]: web_sys::Node
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  pub node: web_sys::Node,
}

#[cfg(all(target_arch = "wasm32", feature = "web"))]
impl Drop for Comment {
  fn drop(&mut self) {
    self.node.unchecked_ref::<web_sys::Element>().remove();
  }
}

impl Comment {
  pub fn new(content: &str) -> Self {
    #[cfg(debug_assertions)]
    assert!(
      !content.contains("-->",),
      "`-->` is not allowed in comment content, as this would preemptively \
       close the comment"
    );

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let node = gloo::utils::document()
      .create_comment(&format!(" {content} "))
      .unchecked_into::<web_sys::Node>();

    Self {
      content: content.to_owned(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      node,
    }
  }
}

/// Represents a custom component.
pub(crate) struct Component<Msg> {
  /// The name of the component.
  name: String,
  #[cfg(all(target_arch = "wasm32", feature = "web"))]
  fragment: web_sys::DocumentFragment,
  /// The opening component delimeter.
  ///
  /// This is used to quickly find the boundary of the component
  /// and will look something like this:
  /// <!-- <ComponentName> --> <-- this is the opening comment
  /// /* children */
  /// <!-- </ComponentName> -->
  pub opening: Comment,
  pub children: Rc<RefCell<Vec<View<Msg>>>>,
  /// The closing component delimiter.
  ///
  /// This is used to quickly find the boundary of the component
  /// and will look something like this:
  /// <!-- <ComponentName> -->
  /// /* children */
  /// <!-- </ComponentName> --> <-- this is the closing comment
  closing: Comment,
}

impl<Msg> fmt::Display for Component<Msg> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let Self {
      opening,
      children,
      closing,
      ..
    } = self;

    opening.fmt(f)?;

    for child in children.borrow().deref() {
      child.fmt(f)?;
    }

    closing.fmt(f)
  }
}

impl<Msg> Component<Msg> {
  pub fn new(name: &str) -> Self {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    let fragment = gloo::utils::document().create_document_fragment();

    let opening = Comment::new(&format!("<{name}>"));
    let closing = Comment::new(&format!("</{name}>"));

    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
      fragment.append_child(&opening.node).unwrap();
      fragment.append_child(&closing.node).unwrap();
    }

    Self {
      name: name.to_owned(),
      #[cfg(all(target_arch = "wasm32", feature = "web"))]
      fragment,
      opening,
      children: Default::default(),
      closing,
    }
  }
}

#[cfg(debug_assertions)]
fn assert_tag_name_is_valid(name: &str) {
  assert!(!name.is_empty(), "tag name must not be empty");
  assert!(
    name.chars().all(|c| if c.is_ascii() {
      c.is_ascii_lowercase()
    } else {
      true
    }),
    "tag names must not contain ASCII upercase letters"
  );
  assert!(
    name.split_whitespace().count() == 1,
    "whitespace is not allowed in tag names"
  );
  assert!(
    name.chars().all(|c| {
      if c.is_ascii() {
        matches!(c, 'a'..='z' | '0'..='9')
      } else {
        true
      }
    }),
    "all tag name ASCII characters must be `a-z` and `0-9`"
  );
}
