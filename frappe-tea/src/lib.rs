#![feature(once_cell, trait_alias)]

#[macro_use]
extern crate async_trait;
#[allow(unused_imports)]
#[macro_use]
extern crate static_assertions;

use std::{
    fmt,
    future::Future,
    lazy::SyncOnceCell,
    marker::PhantomData,
    sync::{atomic, Arc, Mutex, Weak},
};

use utils::spawn;

#[macro_use]
mod utils;
mod runtime;
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
pub trait Node<Msg> {}

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
    pub async fn new<MF, VF, VFut, const N: usize>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Self
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M) -> VFut,
        VFut: Future<Output = [DynNode<Msg>; N]>,
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
        // Get the model
        let mut model = self
            .model
            .lock()
            .unwrap()
            .take()
            .expect("model to not be taken");

        let cmd = (self.update)(&mut model, msg).await;

        // Return model
        *self.model.lock().unwrap() = Some(model);

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
    async fn new<MF, VF, VFut, const N: usize>(
        target: &str,
        initial_model: MF,
        update: UF,
        view: VF,
    ) -> Arc<Self>
    where
        MF: FnOnce() -> (M, Option<DynCmd<Msg>>),
        VF: FnOnce(&M) -> VFut,
        VFut: Future<Output = [DynNode<Msg>; N]>,
    {
        let (model, cmd) = initial_model();

        let children = view(&model).await;

        let this = Arc::new(Self {
            model: Mutex::new(Some(model)),
            update,
            root: SyncOnceCell::new(),
            pending_cmds: atomic::AtomicUsize::new(0),
        });

        let msg_dispatcher_weak =
            Arc::downgrade(&this) as Weak<dyn DispatchMsg<Msg>>;

        let root_node = render(target, children, msg_dispatcher_weak).await;

        if let Some(cmd) = cmd {
            spawn(this.clone().perform_cmd(cmd));
        }

        this.root.set(root_node).unwrap();

        this
    }
}

pub struct NodeTree<Msg> {
    _phantom: PhantomData<Msg>,
}

assert_impl_all!(NodeTree<()>: Send);

impl<Msg> fmt::Debug for NodeTree<Msg> {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

// =============================================================================
//                            Functions
// =============================================================================

async fn render<Msg, const N: usize>(
    _target: &str,
    _children: [DynNode<Msg>; N],
    _msg_dispatcher_weak: Weak<dyn DispatchMsg<Msg>>,
) -> NodeTree<Msg> {
    todo!()
}
