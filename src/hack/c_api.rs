extern crate alloc;

use {
    core::time::Duration,
    alloc::sync::Arc,
    std::{
        sync::{LazyLock, OnceLock},
        thread::{self, JoinHandle},
    },
    super::Result,
    tokio::runtime::Runtime,
};

pub fn runtime() -> Result<&'static Arc<Runtime>> {
    static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();
    static SETUP: LazyLock<JoinHandle<()>> = LazyLock::new(|| thread::spawn(|| {
        RUNTIME.get_or_init(|| Runtime::new().map(|r| Arc::new(r)));
    }));

    while SETUP.is_finished() == false {
        thread::sleep(Duration::from_millis(10));
    }
    RUNTIME.get().ok_or_else(|| err!("Runtime has not been made"))?.as_ref().map_err(|e| err!("{e}"))
}
