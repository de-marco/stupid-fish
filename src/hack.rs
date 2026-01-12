extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::Result,
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
    tokio::{
        net::UnixListener,
        runtime::Runtime,
    },
};

pub static MAIN_THREAD_ID: LazyLock<ThreadId> = LazyLock::new(|| thread::current().id());

static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();

pub fn setup() {
    __info!("Main thread ID: {:?}\n", *MAIN_THREAD_ID);
    thread::spawn(|| RUNTIME.get_or_init(|| {
        if thread::current().id() != *MAIN_THREAD_ID {
            Runtime::new().map(|r| Arc::new(r))
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
    const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";

    let raw_address = format!("{process_id}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{id}", process_id=process::id(), id=id.as_ref());
    let address = SocketAddr::from_abstract_name(&raw_address)?;
    __info!("-> {raw_address}\n");

    let listener = std::os::unix::net::UnixListener::bind_addr(&address)?;
    listener.set_nonblocking(true)?;
    listener.try_into()
}
