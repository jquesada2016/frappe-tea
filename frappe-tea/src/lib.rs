#![feature(once_cell, trait_alias)]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate educe;
#[macro_use]
extern crate static_assertions;

use futures::lock::{Mutex, MutexGuard};
use std::{
    collections::HashMap,
    fmt,
    future::Future,
    lazy::SyncOnceCell,
    sync::{atomic, Arc, Weak},
};
#[cfg(target_arch = "wasm32")]
use utils::is_browser;
use utils::spawn;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[macro_use]
mod utils;
pub mod html;
pub mod testing;

pub type DynNode<Msg> = Box<dyn Node<Msg> + Send>;
pub type DynCmd<Msg> = Box<dyn Cmd<Msg> + Send>;

// =============================================================================
//                              Traits
// =============================================================================

#[async_trait]
pub trait Cmd<Msg> {
    async fn perform_cmd(self: Box<Self>) -> Option<Msg>;
}

#[async_trait]
trait DispatchMsg<Msg> {
    async fn dispatch_msg(self: Arc<Self>, msg: Msg);
}

#[async_trait]
pub trait IntoNode<Msg> {
    async fn into_node(self) -> DynNode<Msg>;
}

#[async_trait]
pub trait Node<Msg> {
    fn node(&self) -> &NodeKind;

    fn node_mut(&mut self) -> &mut NodeKind;

    fn cx(&self) -> &Context<Msg>;

    fn set_ctx(&mut self, cx: Context<Msg>);

    fn children(&self) -> ChildrenRef<Msg>;

    fn children_mut(&mut self) -> ChildrenMut<Msg>;

    #[track_caller]
    fn append_child(&mut self, child: DynNode<Msg>);

    fn clear_children(&mut self);
}

assert_obj_safe!(Node<()>);

#[async_trait]
trait Runtime<Msg> {
    async fn dispatch_msg(self: Arc<Self>, msg: Msg);

    async fn perform_cmd(self: Arc<Self>, cmd: DynCmd<Msg>);
}

// =============================================================================
//                           Structs and Impls
// =============================================================================

pub struct AppElement<M, UF, Msg>(Arc<AppEl<M, UF, Msg>>);

assert_impl_all!(AppElement<(), fn(&mut ()) -> Option<DynCmd<()>>, ()>: Send);

impl<M, UF, Msg, Fut> AppElement<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<DynCmd<Msg>>> + Send,
    Msg: Send + 'static,
{
    pub async fn new<MF, VF, VFut>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Self
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M, Context<Msg>) -> VFut,
        VFut: Future<Output = DynNode<Msg>>,
    {
        Self(AppEl::new(target, initial_model, update, view).await)
    }
}

struct AppEl<M, UF, Msg> {
    model: Mutex<Option<M>>,
    update: UF,
    root: SyncOnceCell<NodeTree<Msg>>,
    pending_cmds: atomic::AtomicUsize,
}

assert_impl_all!(AppEl<(), fn(&mut ()) -> Option<DynCmd<()>>, ()>: Send);

unsafe impl<M, UF, Msg> Sync for AppEl<M, UF, Msg>
where
    M: 'static,
    UF: Send + Sync,
    Msg: 'static,
{
}

#[async_trait]
impl<M, UF, Msg, Fut> DispatchMsg<Msg> for AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<DynCmd<Msg>>> + Send,
    Msg: Send + 'static,
{
    async fn dispatch_msg(self: Arc<Self>, msg: Msg) {
        Runtime::dispatch_msg(self, msg).await;
    }
}

#[async_trait]
impl<M, UF, Msg, Fut> Runtime<Msg> for AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<DynCmd<Msg>>> + Send,
    Msg: Send + 'static,
{
    async fn dispatch_msg(self: Arc<Self>, msg: Msg) {
        let mut model_lock = self.model.lock().await;

        // Get the model
        let mut model = model_gaurd.take().expect("model to not be taken");

        let cmd = (self.update)(&mut model, msg).await;

        // Return model
        *model_lock = Some(model);

        if let Some(cmd) = cmd {
            spawn(self.perform_cmd(cmd));
        }
    }

    async fn perform_cmd(self: Arc<Self>, cmd: DynCmd<Msg>) {
        // Increment pending cmds count
        self.pending_cmds.fetch_add(1, atomic::Ordering::SeqCst);

        let msg = cmd.perform_cmd().await;

        if let Some(msg) = msg {
            Runtime::dispatch_msg(self.clone(), msg).await;
        }

        // Decrement pending cmds count
        self.pending_cmds.fetch_sub(1, atomic::Ordering::SeqCst);
    }
}

impl<M, UF, Msg, Fut> AppEl<M, UF, Msg>
where
    M: Send + 'static,
    UF: Fn(&mut M, Msg) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<DynCmd<Msg>>> + Send,
    Msg: Send + 'static,
{
    async fn new<MF, VF, VFut>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Arc<Self>
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M, Context<Msg>) -> VFut,
        VFut: Future<Output = DynNode<Msg>>,
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
            msg_dispatcher: SyncOnceCell::from(msg_dispatcher_weak.clone()),
            ..Default::default()
        };

        let children =
            view(this.model.lock().unwrap().as_ref().unwrap(), cx).await;

        let root_node = render(target, children, msg_dispatcher_weak).await;

        if let Some(cmd) = cmd {
            spawn(this.clone().perform_cmd(cmd));
        }

        this.root.set(root_node).unwrap();

        this
    }
}

#[derive(Educe)]
#[educe(Deref)]
pub struct ChildrenRef<'a, Msg>(MutexGuard<'a, Vec<DynNode<Msg>>>);

#[derive(Educe)]
#[educe(Deref, DerefMut)]
pub struct ChildrenMut<'a, Msg>(MutexGuard<'a, Vec<DynNode<Msg>>>);

#[derive(Clone, Educe)]
#[educe(Default)]
pub struct Context<Msg> {
    id: Id,
    msg_dispatcher: SyncOnceCell<Weak<dyn DispatchMsg<Msg> + Send + Sync>>,
}

/// Represents a topologically unique and stable ID in a node tree.
#[derive(Clone, Default)]
struct Id(usize, usize, usize, Option<String>);

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
        self.3.as_deref()
    }

    fn _set_sum(&mut self, sum: usize) {
        self.0 += sum;
    }

    fn _set_depth(&mut self, depth: usize) {
        self.1 += depth;
    }

    fn _set_index(&mut self, index: usize) {
        self.2 += index;
    }

    fn _set_custom_id(&mut self, custom_id: String) {
        self.3 = Some(custom_id);
    }

    fn _set_id(&mut self, parent_id: &Id, index: usize) {
        self._set_sum(parent_id.0 + parent_id.1 + parent_id.2);

        self._set_depth(parent_id.1 + 1);

        self._set_index(index);
    }
}

#[allow(dead_code)]
pub enum NodeKind {
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
    },
    Tag {
        name: String,
        #[cfg(target_arch = "wasm32")]
        node: Option<WasmValue<web_sys::Node>>,
        attributes: HashMap<String, String>,
        properties: HashMap<String, String>,
    },
    Text(
        String,
        #[cfg(target_arch = "wasm32")] Option<WasmValue<web_sys::Text>>,
    ),
}

assert_impl_all!(NodeKind: Send);

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

    #[track_caller]
    fn new_tag(tag_name: &str) -> Self {
        let name = tag_name.to_string();

        #[cfg(target_arch = "wasm32")]
        let node = {
            let tag_node = gloo::utils::document()
                .create_element(&name)
                .unwrap_or_else(|_| panic!("failed to create element `{name}`"))
                .unchecked_into();

            Some(WasmValue(tag_node))
        };

        Self::Tag {
            name,
            #[cfg(target_arch = "wasm32")]
            node,
            attributes: HashMap::new(),
            properties: HashMap::new(),
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
}

pub struct NodeTree<Msg> {
    cx: Context<Msg>,
    node: NodeKind,
    children: Mutex<Vec<DynNode<Msg>>>,
}

assert_impl_all!(NodeTree<()>: Send);

impl<Msg> fmt::Debug for NodeTree<Msg> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[async_trait]
impl<Msg> IntoNode<Msg> for NodeTree<Msg>
where
    Msg: 'static,
{
    async fn into_node(self) -> DynNode<Msg> {
        Box::new(self)
    }
}

impl<Msg> NodeTree<Msg> {
    pub fn new_component(name: &str) -> Self {
        Self {
            cx: Context::default(),
            node: NodeKind::new_component(name),
            children: Mutex::new(vec![]),
        }
    }

    pub fn new_tag(tag_name: &str) -> Self {
        Self {
            cx: Context::default(),
            node: NodeKind::new_tag(tag_name),
            children: Mutex::new(vec![]),
        }
    }

    pub fn new_text(text: &str) -> Self {
        Self {
            cx: Context::default(),
            node: NodeKind::new_text(text),
            children: Mutex::new(vec![]),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn from_raw_node(node: web_sys::Node) -> Self {
        Self {
            cx: Context::default(),
            node: NodeKind::from_raw_node(node),
            children: Mutex::new(vec![]),
        }
    }
}

/// Removes the node from the DOM.
#[cfg(target_arch = "wasm32")]
impl<Msg> Drop for NodeTree<Msg> {
    // TODO: Batch the drops and synchronize with `requestAnimationFrame`
    fn drop(&mut self) {
        match &self.node {
            NodeKind::Component {
                opening_marker,
                state_marker,
                closing_marker,
                ..
            } => {
                if let Some(opening_marker) = opening_marker {
                    opening_marker.unchecked_ref::<web_sys::Element>().remove();
                }

                if let Some(state_marker) = state_marker {
                    state_marker.unchecked_ref::<web_sys::Element>().remove();
                }

                if let Some(closing_marker) = closing_marker {
                    closing_marker.unchecked_ref::<web_sys::Element>().remove();
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

#[async_trait]
impl<Msg> Node<Msg> for NodeTree<Msg> {
    fn node(&self) -> &NodeKind {
        &self.node
    }

    fn node_mut(&mut self) -> &mut NodeKind {
        &mut self.node
    }

    fn cx(&self) -> &Context<Msg> {
        &self.cx
    }

    fn set_ctx(&mut self, cx: Context<Msg>) {
        self.cx = cx;
    }

    async fn children(&self) -> ChildrenRef<Msg> {
        ChildrenRef(self.children.lock().unwrap())
    }

    async fn children_mut(&mut self) -> ChildrenMut<Msg> {
        ChildrenMut(self.children.lock().unwrap())
    }

    fn append_child(&mut self, child: DynNode<Msg>) {
        // We only need to insert items into the DOM when we are running
        // in the browser
        // app
        #[cfg(target_arch = "wasm32")]
        if is_browser() {
            match &self.node {
                // We don't have to insert anything here, because there is no
                // actual node for us to insert into. Components are flat,
                // i.e., they do not have an inherent parent, and therefore
                // require a `tag` parent to exist. We will insert it later,
                // once we have a parent which is a `tag` variant
                NodeKind::Component { .. } => { /* do nothing */ }
                NodeKind::Tag {
                    node: parent_node, ..
                } => match &child.node() {
                    // Since components don't have an actual tag, we
                    // need to recursively insert all component children
                    NodeKind::Component { .. } => todo!(),
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

        self.children_mut().push(child);
    }

    fn clear_children(&mut self) {
        self.children_mut().clear();
    }
}

/// Wrapper to mark any JavaScript value as thread safe.
///
/// # Safety
/// This is only safe if you can guarantee the value will only ever be accessed on the main
/// thread. For the most part, this means, if you are running in the browser, then it is
/// safe to access this value (for now).
#[derive(Educe)]
#[educe(Deref, DerefMut)]
pub struct WasmValue<T>(T);

unsafe impl<T> Send for WasmValue<T> {}
unsafe impl<T> Sync for WasmValue<T> {}

// =============================================================================
//                            Functions
// =============================================================================

#[cfg(target_arch = "wasm32")]
async fn render<Msg>(
    target: &str,
    child: DynNode<Msg>,
    msg_dispatcher_weak: Weak<dyn DispatchMsg<Msg>>,
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

        target.append_child(child);

        target
    } else {
        todo!()
    }
}

api_planning! {
    for child in children {
        child = child().await;


    }

    async fn view(model: &Model, cx: Context) -> DynNode<Msg> {
        // 0-0-0
        Fragment::new()
                .cx(cx)
                .child(|cx| async {
                    div()
                        .cx(cx)
                        .child(|cx| async { h1().ctx(cx).into_node().await })
                        .into_node()
                        .await
                })
                .into_node()
                .await
    }
}
