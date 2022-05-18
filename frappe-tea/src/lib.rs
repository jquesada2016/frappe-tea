#![feature(once_cell)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate educe;
#[macro_use]
#[allow(unused_imports)]
extern crate log;
#[macro_use]
extern crate static_assertions;

#[macro_use]
mod utils;
pub mod components;
pub mod html;
pub mod reactive;
pub mod testing;

use std::{
    collections::HashMap,
    fmt,
    lazy::SyncOnceCell,
    ops::Deref,
    sync::{
        atomic::{self, AtomicBool, AtomicUsize},
        Arc, Mutex, Weak,
    },
};
#[cfg(target_arch = "wasm32")]
use utils::is_browser;
use utils::spawn;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

pub type DynCmd<Msg> = Box<dyn Cmd<Msg> + Send>;

// =============================================================================
//                              Modules
// =============================================================================

pub mod prelude {
    pub use super::*;
    pub use components::*;
    pub use reactive::*;
}

// =============================================================================
//                              Traits
// =============================================================================

/// Trait to allow async side-effects.
#[async_trait]
pub trait Cmd<Msg> {
    /// Side-effect to perform when the `cmd` is executed.
    async fn perform_cmd(self: Box<Self>) -> Option<Msg>;
}

/// This trait allows dispatching messages to the app's `update` function.
trait DispatchMsg<Msg> {
    /// Dispatch message.
    fn dispatch_msg(self: Arc<Self>, msg: Msg);
}

/// Trait for converting data into a [`NodeTree`].
pub trait IntoNode<Msg> {
    /// Converts `Self` into [`NodeTree`].
    fn into_node(self) -> NodeTree<Msg>;
}

#[async_trait]
trait Runtime<Msg> {
    fn dispatch_msg(self: Arc<Self>, msg: Msg);

    async fn perform_cmd(self: Arc<Self>, cmd: DynCmd<Msg>);
}

// =============================================================================
//                           Structs and Impls
// =============================================================================

/// Main entry point for isomorphic apps.
pub struct AppElement<M, UF, Msg>(Arc<AppEl<M, UF, Msg>>);

assert_impl_all!(AppElement<(), fn(&mut ()) -> Option<DynCmd<()>>, ()>: Send);

impl<M, UF, Msg> AppElement<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Option<DynCmd<Msg>> + Send + 'static,
    Msg: Send + 'static,
{
    pub fn new<MF, VF, N>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Self
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M, &Context<Msg>) -> N,
        N: IntoNode<Msg>,
    {
        Self(AppEl::new(target, initial_model, update, view))
    }
}

/// This struct exists solely to allow creating a shared reference to an [`AppElement`] instance.
struct AppEl<M, UF, Msg> {
    model: Mutex<Option<M>>,
    update: UF,
    // We need to hold onto the root so it doesn't drop and undo all our hard work
    root: SyncOnceCell<NodeTree<Msg>>,
    /// Counter designed to keep tabs on pending commands preventing us from rendering
    /// to a string.
    pending_cmds: atomic::AtomicUsize,
}

assert_impl_all!(AppEl<(), fn(&mut ()) -> Option<DynCmd<()>>, ()>: Send);

// Safety:
// This is safe because access to the update fn is gated by a mutex lock
// on model.
unsafe impl<M, UF, Msg> Sync for AppEl<M, UF, Msg>
where
    M: 'static,
    UF: Send + 'static,
    Msg: 'static,
{
}

impl<M, UF, Msg> DispatchMsg<Msg> for AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Option<DynCmd<Msg>> + Send + 'static,
    Msg: Send + 'static,
{
    fn dispatch_msg(self: Arc<Self>, msg: Msg) {
        Runtime::dispatch_msg(self, msg);
    }
}

#[async_trait]
impl<M, UF, Msg> Runtime<Msg> for AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Option<DynCmd<Msg>> + Send + 'static,
    Msg: Send + 'static,
{
    fn dispatch_msg(self: Arc<Self>, msg: Msg) {
        let mut model_lock = self.model.lock().unwrap();

        // Get the model
        let mut model = model_lock.take().expect("model to not be taken");

        let cmd = (self.update)(&mut model, msg);

        // Return model
        *model_lock = Some(model);

        drop(model_lock);

        if let Some(cmd) = cmd {
            spawn(self.perform_cmd(cmd));
        }
    }

    async fn perform_cmd(self: Arc<Self>, cmd: DynCmd<Msg>) {
        // Increment pending cmds count
        self.pending_cmds.fetch_add(1, atomic::Ordering::SeqCst);

        let msg = cmd.perform_cmd().await;

        if let Some(msg) = msg {
            Runtime::dispatch_msg(self.clone(), msg);
        }

        // Decrement pending cmds count
        self.pending_cmds.fetch_sub(1, atomic::Ordering::SeqCst);
    }
}

impl<M, UF, Msg> AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Option<DynCmd<Msg>> + Send + 'static,
    Msg: Send + 'static,
{
    fn new<MF, VF, N>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Arc<Self>
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M, &Context<Msg>) -> N,
        N: IntoNode<Msg>,
    {
        let (model, cmd) = initial_model();

        let this = Arc::new(Self {
            model: Mutex::new(Some(model)),
            update,
            root: SyncOnceCell::new(),
            pending_cmds: atomic::AtomicUsize::new(0),
        });

        let msg_dispatcher_weak =
            Arc::downgrade(&this) as Weak<dyn DispatchMsg<Msg> + Send + Sync>;

        let cx = Context {
            msg_dispatcher: SyncOnceCell::from(msg_dispatcher_weak),
            ..Default::default()
        };

        let child =
            view(this.model.lock().unwrap().as_ref().unwrap(), &cx).into_node();

        let root_node = render(target, child, cx);

        debug!("{root_node}");

        if let Some(cmd) = cmd {
            spawn(this.clone().perform_cmd(cmd));
        }

        this.root.set(root_node).unwrap();

        this
    }
}

/// Type of insertion operation when inserting a node relative to another in the
/// DOM.
#[derive(Clone, Copy)]
enum InsertMode {
    Append,
    Before,
}

/// Data structure which will hold a [`NodeTree`]'s children.
///
/// This struct is meant to be held within an [`Arc`].
#[derive(Educe)]
#[educe(Default)]
struct Children<Msg> {
    /// Context belonging to the parent. This field is in the children because we need
    /// to be able to get a shared reference to it when accessing children.
    cx: SyncOnceCell<Context<Msg>>,
    children: Mutex<Vec<NodeTree<Msg>>>,
}

impl<Msg> Children<Msg> {
    #[track_caller]
    fn cx(&self) -> &Context<Msg> {
        self.cx
            .get()
            .expect("attempted to get context before being set")
    }

    #[track_caller]
    fn _msg_dispatcher(&self) -> Weak<dyn DispatchMsg<Msg> + Send> {
        self.cx()
            .msg_dispatcher
            .get()
            .expect(
                "attempted to get message dispatcher before connecting to \
                     the runtime",
            )
            .clone()
    }

    #[track_caller]
    fn set_cx(&self, parent_cx: &Context<Msg>) {
        let index =
            parent_cx.next_index.fetch_add(1, atomic::Ordering::Relaxed);

        let cx = Context::from_parent_cx(parent_cx, index);

        self.cx
            .set(cx)
            .ok()
            .expect("cannot set context more than once");
    }

    fn append(&self, this: &NodeKind, child: NodeTree<Msg>) {
        // We only need to insert items into the DOM when we are running
        // in the browser
        #[cfg(target_arch = "wasm32")]
        if is_browser() {
            this.append(&child);
        }

        self.children.lock().unwrap().push(child);
    }

    /// # Panics
    /// This function will panic when browser API's are not available.
    #[cfg(target_arch = "wasm32")]
    #[track_caller]
    fn recurseively_append_component_children(
        this: &web_sys::Node,
        child: &NodeTree<Msg>,
        insert_mode: InsertMode,
    ) {
        const FAILED_APPEND: &str = "failed to append node";

        match &child.node {
            NodeKind::Tag { node, .. } => match insert_mode {
                InsertMode::Append => {
                    this.append_child(node.as_ref().unwrap().deref())
                        .expect(FAILED_APPEND);
                }
                InsertMode::Before => this
                    .unchecked_ref::<web_sys::Element>()
                    .before_with_node_1(node.as_ref().unwrap().deref())
                    .expect(FAILED_APPEND),
            },
            NodeKind::Component {
                opening_marker,
                closing_marker,
                ..
            } => match insert_mode {
                InsertMode::Append => {
                    this.append_child(
                        opening_marker
                            .as_ref()
                            .unwrap()
                            .deref()
                            .unchecked_ref::<web_sys::Node>(),
                    )
                    .expect(FAILED_APPEND);

                    for child in child.children.children.lock().unwrap().iter()
                    {
                        Self::recurseively_append_component_children(
                            this,
                            child,
                            insert_mode,
                        );
                    }

                    this.append_child(
                        closing_marker
                            .as_ref()
                            .unwrap()
                            .deref()
                            .unchecked_ref::<web_sys::Node>(),
                    )
                    .expect(FAILED_APPEND);
                }
                InsertMode::Before => {
                    this.unchecked_ref::<web_sys::Element>()
                        .before_with_node_1(
                            opening_marker
                                .as_ref()
                                .unwrap()
                                .deref()
                                .unchecked_ref::<web_sys::Node>(),
                        )
                        .expect(FAILED_APPEND);

                    for child in child.children.children.lock().unwrap().iter()
                    {
                        Self::recurseively_append_component_children(
                            this,
                            child,
                            insert_mode,
                        );
                    }

                    this.unchecked_ref::<web_sys::Element>()
                        .before_with_node_1(
                            closing_marker
                                .as_ref()
                                .unwrap()
                                .deref()
                                .unchecked_ref::<web_sys::Node>(),
                        )
                        .expect(FAILED_APPEND);
                }
            },
            NodeKind::Text(_, node) => match insert_mode {
                InsertMode::Append => {
                    this.append_child(node.as_ref().unwrap().deref())
                        .expect(FAILED_APPEND);
                }
                InsertMode::Before => this
                    .unchecked_ref::<web_sys::Element>()
                    .before_with_node_1(node.as_ref().unwrap().deref())
                    .expect(FAILED_APPEND),
            },
        }
    }

    fn clear(&self) {
        // We need to reset next_index to keep the id generation
        // consistant
        self.cx().next_index.store(0, atomic::Ordering::Relaxed);

        self.children.lock().unwrap().clear();
    }
}

/// Context information needed by the node.
#[derive(Educe)]
#[educe(Default, Clone)]
pub struct Context<Msg> {
    /// A structurally-stable unique [`Id`], which will always
    /// produce the same [`Id`] for the same node tree.
    id: Id,
    /// If true, the current node is dynamic, and must therefore be hydrated from the
    /// DOM when loaded in the browser. Otherwise, there is no need to try and
    /// hydrate the node, as it will never change.
    dynamic: Arc<AtomicBool>,
    /// Message dispatcher.
    msg_dispatcher: SyncOnceCell<Weak<dyn DispatchMsg<Msg> + Send + Sync>>,
    /// This is used to aid in generating unique [`Id`]'s.
    next_index: Arc<AtomicUsize>,
}

impl<Msg> Context<Msg> {
    fn from_parent_cx(parent_cx: &Context<Msg>, index: usize) -> Self {
        let mut this = Context {
            msg_dispatcher: parent_cx.msg_dispatcher.clone(),
            ..Default::default()
        };

        this.id.set_id(&parent_cx.id, index);

        this
    }
}

/// Event listener
pub struct EventHandler {
    #[cfg(target_arch = "wasm32")]
    _handler: Option<gloo::events::EventListener>,
    /// Used for debugging event listeners.
    location: &'static std::panic::Location<'static>,
}

assert_impl_all!(EventHandler: Send);

/// # Safety
/// This is only safe if [`EventHandler`] is not accessed from another thread
/// in wasm.
unsafe impl Send for EventHandler {}

impl Clone for EventHandler {
    fn clone(&self) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            _handler: None,
            location: self.location,
        }
    }
}

/// Represents a topologically unique and stable ID in a node tree.
///
/// The positional parameters are as follows:
/// - 0: sum of parent's `id` parts 0, 1, and 2
/// - 1: depth in the tree
/// - 2: index of child with respect to parent
/// - 3: custom HTML `id` attribute
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Id(usize, usize, usize, SyncOnceCell<String>);

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(id) = self.custom_id() {
            f.write_str(id)
        } else {
            f.write_fmt(format_args!(
                "{}-{}-{}",
                self.sum(),
                self.depth(),
                self.index()
            ))
        }
    }
}
impl PartialEq<(usize, usize, usize)> for Id {
    fn eq(&self, rhs: &(usize, usize, usize)) -> bool {
        self.sum() == rhs.0 && self.depth() == rhs.1 && self.index() == rhs.2
    }
}

impl Id {
    fn sum(&self) -> usize {
        self.0
    }

    fn depth(&self) -> usize {
        self.1
    }

    fn index(&self) -> usize {
        self.2
    }

    fn custom_id(&self) -> Option<&str> {
        self.3.get().map(String::as_str)
    }

    fn set_sum(&mut self, sum: usize) {
        self.0 += sum;
    }

    fn set_depth(&mut self, depth: usize) {
        self.1 += depth;
    }

    fn set_index(&mut self, index: usize) {
        self.2 += index;
    }

    #[track_caller]
    fn _set_custom_id(&self, custom_id: String) {
        self.3.set(custom_id).expect("cannot set id more than once");
    }

    fn set_id(&mut self, parent_id: &Id, index: usize) {
        self.set_sum(parent_id.0 + parent_id.1 + parent_id.2);

        self.set_depth(parent_id.1 + 1);

        self.set_index(index);
    }
}

// /// Throw-away trait to produce a None value with Educe Clone
// trait OptionCloneToNone<T> {
//     fn clone(&self) -> Option<T> {
//         None
//     }
// }

// impl<T> OptionCloneToNone<T> for Option<T> {}

/// Enum of possible node types.
#[derive(Clone)]
pub enum NodeKind {
    /// [`frappe-tea`](self) component.
    Component {
        /// Comment nodes allow for better readability and debugging,
        /// as they allow the user to see what markup belongs to what
        /// component. It also allows extremely efficient insertion
        /// into the DOM without requiring any kind of computation
        /// to figure out where nodes must go relative to any other.
        ///
        /// The reason this is `Option<_>` is because if we are rendering
        /// from a non-browser `wasm32` server, such as `Deno` or `Node.js`,
        /// we cannot use browser API's.
        #[cfg(target_arch = "wasm32")]
        opening_marker: Option<WasmValue<web_sys::Comment>>,

        /// All component-local state will be serialized into data
        /// attributes within an empty `<template />`. Since comments
        /// cannot have attributes, we therefore need a lightweight
        /// tag that will cause as little performance impact as possible.
        /// To the best of my knowledge, the `<template />` fits the bill.
        #[cfg(all(target_arch = "wasm32", feature = "hmr"))]
        state_marker: Option<WasmValue<web_sys::Node>>,

        /// Component name.
        name: String,

        /// When on the server, all local state must be serialized
        /// and send down to the client in order to allow resuming
        /// the app.
        /// When HMR is enabled, we need to constantly be serializing
        /// changes to local state in order to be able to resume from
        /// where we left off the next time the wasm module is loaded.
        #[cfg(any(not(target_arch = "wasm32"), feature = "hmr"))]
        local_state: Vec<String>,

        #[cfg(target_arch = "wasm32")]
        closing_marker: Option<WasmValue<web_sys::Comment>>,
        // /// Used to unsubscribe the component from any subscriptions
        // /// when the component is dropped.
        // /// TODO: make this Send
        // #[educe(Clone(trait = "OptionCloneToNone"))]
        // unsubscriber: Option<Mutex<Box<dyn Unsubscribe + Send>>>,
    },
    /// HTML node.
    Tag {
        name: String,
        #[cfg(target_arch = "wasm32")]
        node: Option<WasmValue<web_sys::Node>>,
        attributes: HashMap<String, String>,
        properties: HashMap<String, String>,
        event_handlers: Vec<EventHandler>,
    },
    /// Text node.
    Text(
        String,
        #[cfg(target_arch = "wasm32")] Option<WasmValue<web_sys::Text>>,
    ),
}

assert_impl_all!(NodeKind: Send);

impl fmt::Debug for NodeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Component { name, .. } => {
                f.debug_tuple("Component").field(name).finish()
            }
            Self::Tag { name, .. } => f.debug_tuple("Tag").field(name).finish(),
            Self::Text(text, ..) => f.debug_tuple("Text").field(text).finish(),
        }
    }
}

impl NodeKind {
    fn new_component(name: &str) -> Self {
        let name = name.to_string();

        #[cfg(target_arch = "wasm32")]
        let (opening_marker, closing_marker) = {
            if is_browser() {
                let opening_marker = gloo::utils::document()
                    .create_comment(&format!(" <{name}> "));
                let closing_marker = gloo::utils::document()
                    .create_comment(&format!(" </{name}> "));

                (
                    Some(WasmValue(opening_marker)),
                    Some(WasmValue(closing_marker)),
                )
            } else {
                (None, None)
            }
        };

        #[cfg(all(target_arch = "wasm32", feature = "hmr"))]
        let state_marker = {
            if is_browser() {
                let template = gloo::utils::document()
                    .create_element("template")
                    .unwrap()
                    .unchecked_into();

                Some(WasmValue(template))
            } else {
                None
            }
        };

        Self::Component {
            #[cfg(target_arch = "wasm32")]
            opening_marker,
            #[cfg(all(target_arch = "wasm32", feature = "hmr"))]
            state_marker,
            name,
            local_state: vec![],
            #[cfg(target_arch = "wasm32")]
            closing_marker,
        }
    }

    /// Creates a new HTML tag.
    /// The `id` is [`None`] iff the node is a static node.
    #[track_caller]
    fn new_tag(tag_name: &str, id: Option<Id>) -> Self {
        let name = tag_name.to_string();

        #[cfg(target_arch = "wasm32")]
        let node = {
            let tag_node = if is_browser() {
                // If SSR is enabled, we need to hydrate the node
                if cfg!(feature = "ssr") {
                    if let Some(id) = id {
                        gloo::utils::document()
                            .get_element_by_id(&id.to_string())
                            .unwrap_or_else(|| {
                                panic!("failed to get element with id {}", id)
                            })
                            .unchecked_into::<web_sys::Node>()
                    } else {
                        gloo::utils::document()
                            .create_element(&name)
                            .unwrap_or_else(|_| {
                                panic!("failed to create element `{name}`")
                            })
                            .unchecked_into()
                    }
                } else {
                    gloo::utils::document()
                        .create_element(&name)
                        .unwrap_or_else(|_| {
                            panic!("failed to create element `{name}`")
                        })
                        .unchecked_into()
                }
            } else {
                gloo::utils::document()
                    .create_element(&name)
                    .unwrap_or_else(|_| {
                        panic!("failed to create element `{name}`")
                    })
                    .unchecked_into()
            };

            Some(WasmValue(tag_node))
        };

        Self::Tag {
            name,
            #[cfg(target_arch = "wasm32")]
            node,
            attributes: HashMap::new(),
            properties: HashMap::new(),
            event_handlers: vec![],
        }
    }

    fn new_text(text: &str) -> Self {
        let text = text.to_string();

        #[cfg(target_arch = "wasm32")]
        let text_node = {
            if is_browser() {
                let text_node = gloo::utils::document().create_text_node(&text);

                Some(WasmValue(text_node))
            } else {
                None
            }
        };

        Self::Text(
            text,
            #[cfg(target_arch = "wasm32")]
            text_node,
        )
    }

    #[cfg(target_arch = "wasm32")]
    fn from_raw_node(node: web_sys::Node) -> Self {
        let (name, text) = if is_browser() {
            let node_name = node.node_name().to_lowercase();

            if node_name == "#text" {
                (None, Some(node.text_content().unwrap_or_default()))
            } else {
                (Some(node_name), None)
            }
        } else {
            unreachable!("where did you get a node frome if not running in the browser???");
        };

        if let Some(name) = name {
            Self::Tag {
                name,
                node: Some(WasmValue(node)),
                attributes: HashMap::new(),
                properties: HashMap::new(),
                event_handlers: vec![],
            }
        } else {
            let text = text.unwrap();

            Self::Text(
                text.clone(),
                Some(WasmValue(
                    gloo::utils::document().create_text_node(&text),
                )),
            )
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn append<Msg>(&self, child: &NodeTree<Msg>) {
        match self {
            // We have to check to see if any of the component parts
            // are inserted in the DOM. If they are, then this means
            // we can proceed as normal, inserting elements before
            // the closing marker. If not then  We don't have to
            // insert anything here, because there is no
            // actual node for us to insert into. Components are flat,
            // i.e., they do not have an inherent parent in the DOM,
            // and therefore require a `tag` parent to exist. We will
            // insert it later, once we have a parent which is a `tag`
            // variant
            NodeKind::Component { closing_marker, .. } => {
                let closing_marker = closing_marker.as_ref().unwrap().deref();

                if closing_marker.is_connected() {
                    match &child.node {
                        Self::Component { .. } => {
                            Children::recurseively_append_component_children(
                                closing_marker,
                                child,
                                InsertMode::Before,
                            );

                            todo!("add InsertBefore");
                        }
                        Self::Tag { node, .. } => closing_marker
                            .unchecked_ref::<web_sys::Element>()
                            .before_with_node_1(node.as_ref().unwrap().deref())
                            .unwrap_or_else(|v| {
                                panic!("failed to prepend node: {:#?}", v)
                            }),
                        Self::Text(_, node) => closing_marker
                            .unchecked_ref::<web_sys::Element>()
                            .before_with_node_1(node.as_ref().unwrap().deref())
                            .unwrap_or_else(|v| {
                                panic!("failed to prepend node: {:#?}", v)
                            }),
                    }
                } else {
                    /* do nothing yet */
                }
            }
            NodeKind::Tag {
                node: parent_node, ..
            } => match &child.node {
                // Since components don't have an actual tag, we
                // need to recursively insert all component children
                NodeKind::Component { .. } => {
                    Children::recurseively_append_component_children(
                        &*parent_node.as_ref().unwrap(),
                        child,
                        InsertMode::Append,
                    )
                }
                NodeKind::Tag {
                    node: child_node, ..
                } => {
                    parent_node
                        .as_ref()
                        .unwrap()
                        .append_child(child_node.as_ref().unwrap())
                        .unwrap();
                }
                NodeKind::Text(_, child_text) => {
                    parent_node
                        .as_ref()
                        .unwrap()
                        .append_child(child_text.as_ref().unwrap())
                        .unwrap();
                }
            },
            NodeKind::Text(..) => panic!("text nodes cannot have children"),
        }
    }
}

/// Represents a single node with all it's children.
pub struct NodeTree<Msg> {
    node: NodeKind,
    children: Arc<Children<Msg>>,
}

assert_impl_all!(NodeTree<()>: Send);

impl<Msg> fmt::Debug for NodeTree<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeTree")
            .field("id", &self.children.cx().id)
            .field("node", &self.node)
            .field("children", &*self.children.children.lock().unwrap())
            .finish()
    }
}
impl<Msg> fmt::Display for NodeTree<Msg> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Every byte saved is a byte to live another day :)

        match &self.node {
            NodeKind::Component { .. } => {
                f.write_fmt(format_args!(
                    r#"<template id="{}o"></template>"#,
                    self.children.cx().id,
                ))?;

                for child in self.children.children.lock().unwrap().iter() {
                    child.fmt(f)?
                }

                f.write_fmt(format_args!(
                    r#"<template id="{}c"></template>"#,
                    self.children.cx().id,
                ))
            }
            NodeKind::Tag {
                name, attributes, ..
            } => {
                if attributes.is_empty() {
                    f.write_fmt(format_args!("<{name}>"))?;
                } else {
                    f.write_fmt(format_args!("<{name} "))?;

                    for (name, val) in attributes {
                        f.write_fmt(format_args!(r#"{name}="{val}""#))?;
                    }

                    f.write_str(">")?;
                }

                for child in self.children.children.lock().unwrap().iter() {
                    child.fmt(f)?;
                }

                f.write_fmt(format_args!("</{name}>"))
            }
            NodeKind::Text(text, ..) => text.fmt(f),
        }
    }
}

impl<Msg> IntoNode<Msg> for NodeTree<Msg>
where
    Msg: 'static,
{
    fn into_node(self) -> NodeTree<Msg> {
        self
    }
}

impl<Msg> NodeTree<Msg> {
    pub fn new_component(name: &str) -> Self {
        Self {
            node: NodeKind::new_component(name),
            children: Arc::new(Children::default()),
        }
    }

    pub fn new_tag(tag_name: &str, cx: &Context<Msg>) -> Self {
        let children = Children::default();

        children.set_cx(cx);

        let cx = children.cx();

        let id = cx.id.to_owned();

        Self {
            node: NodeKind::new_tag(
                tag_name,
                cx.dynamic.load(atomic::Ordering::Relaxed).then_some(id),
            ),
            children: Arc::new(children),
        }
    }

    pub fn new_text(text: &str) -> Self {
        Self {
            node: NodeKind::new_text(text),
            children: Arc::new(Children::default()),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(node: web_sys::Node) -> Self {
        Self {
            node: NodeKind::from_raw_node(node),
            children: Arc::new(Children::default()),
        }
    }

    #[track_caller]
    pub fn append_child(&mut self, child: NodeTree<Msg>) {
        self.children.append(&self.node, child);
    }

    pub fn clear_children(&mut self) {
        self.children.clear();
    }
}

/// Removes the node from the DOM.
#[cfg(target_arch = "wasm32")]
impl<Msg> Drop for NodeTree<Msg> {
    // TODO: Batch the drops and synchronize with `requestAnimationFrame`
    fn drop(&mut self) {
        // We only want to drop if we aren't the root node, since the
        // root node was provided externally, and that would just be rude

        if self.children.cx().id != (0, 0, 0) {
            match &self.node {
                NodeKind::Component {
                    opening_marker,
                    state_marker,
                    closing_marker,
                    ..
                } => {
                    if let Some(opening_marker) = opening_marker {
                        opening_marker
                            .unchecked_ref::<web_sys::Element>()
                            .remove();
                    }

                    if let Some(state_marker) = state_marker {
                        state_marker
                            .unchecked_ref::<web_sys::Element>()
                            .remove();
                    }

                    if let Some(closing_marker) = closing_marker {
                        closing_marker
                            .unchecked_ref::<web_sys::Element>()
                            .remove();
                    }
                }
                NodeKind::Tag { node, .. } => {
                    if let Some(node) = node {
                        node.unchecked_ref::<web_sys::Element>().remove()
                    }
                }
                NodeKind::Text(_, text) => {
                    if let Some(text) = text {
                        text.unchecked_ref::<web_sys::Element>().remove();
                    }
                }
            }
        }
    }
}

/// Wrapper to mark any JavaScript value as thread safe.
///
/// # Safety
/// This is only safe if you can guarantee the value will only ever be accessed on the main
/// thread. For the most part, this means, if you are running in the browser, then it is
/// safe to access this value (for now).
#[derive(Educe, Clone)]
#[educe(Deref, DerefMut)]
pub struct WasmValue<T>(T);

unsafe impl<T> Send for WasmValue<T> {}
unsafe impl<T> Sync for WasmValue<T> {}

// =============================================================================
//                            Functions
// =============================================================================

/// Renders the initial state of the app.
#[cfg(target_arch = "wasm32")]
fn render<Msg>(
    target: &str,
    child: NodeTree<Msg>,
    cx: Context<Msg>,
) -> NodeTree<Msg> {
    // Get the target node
    if is_browser() {
        let target = gloo::utils::document()
            .query_selector(target)
            .unwrap_or_else(|_| {
                panic!("failed to query the document for `{target}`")
            })
            .unwrap_or_else(|| {
                panic!("could not find the node with the query `{target}`")
            });

        // Intern the target node
        let mut target = NodeTree::from_raw_node(target.unchecked_into());

        // We need to manually set the cx so that the root has id
        // 0-0-0
        target.children.cx.set(cx).ok().unwrap();

        target.append_child(child);

        target
    } else {
        todo!()
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn render<Msg>(_target: &str, _child: NodeTree<Msg>) -> NodeTree<Msg> {
    todo!()
}
