extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        os::{
            linux::net::SocketAddrExt,
            unix::net::SocketAddr,
        },
        path::MAIN_SEPARATOR,
        process,
        sync::{LazyLock, OnceLock},
        thread::{self, ThreadId},
    },
    fake_log::__info,
    lacol_rpc::debts::nairud::MapKind,
    tokio::{
        net::UnixListener,
        runtime::Runtime,
    },
};

mod proc_man;

pub type Result<T> = std::io::Result<T>;

const MAP_KIND: MapKind = MapKind::HashMap;

pub static MAIN_THREAD_ID: LazyLock<ThreadId> = LazyLock::new(|| thread::current().id());

static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();

pub fn setup() {
    __info!("Main thread ID: {:?}\n", *MAIN_THREAD_ID);
    thread::spawn(|| RUNTIME.get_or_init(|| {
        if thread::current().id() != *MAIN_THREAD_ID {
            let runtime = Runtime::new().map(|r| Arc::new(r));
            if let Ok(runtime) = &runtime {
                runtime.spawn(proc_man::Task::Finished.start_server());
            }
            runtime
        } else {
            Err(err!("Cannot make new Runtime in main thread"))
        }
    }));

    crate::builtins::cd::history::setup();
}

pub fn runtime() -> Result<&'static Arc<Runtime>> {
    RUNTIME.get().ok_or_else(|| err!("Runtime has not been made"))?.as_ref().map_err(|e| err!("{e}"))
}

pub fn bind<S>(id: S) -> Result<UnixListener> where S: AsRef<str> {
    let listener = std::os::unix::net::UnixListener::bind_addr(&SocketAddr::from_abstract_name(&form_address(id))?)?;
    listener.set_nonblocking(true)?;
    listener.try_into()
}

fn form_address<S>(id: S) -> String where S: AsRef<str> {
    form_address_with(process::id(), id)
}

fn form_address_with<S>(process_id: u32, id: S) -> String where S: AsRef<str> {
    const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";

    let result = format!("{process_id}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{id}", id=id.as_ref());
    __info!("-> {result}\n");

    result
}
