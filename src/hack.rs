extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::Result,
        sync::{LazyLock, OnceLock},
        thread::{self, ThreadId},
    },
    fake_log::__info,
    tokio::runtime::Runtime,
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
