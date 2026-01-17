extern crate alloc;

use {
    core::time::Duration,
    alloc::sync::Arc,
    std::{
        sync::{LazyLock, OnceLock},
        thread::{self, JoinHandle},
        time::Instant,
    },
    super::Result,
    tokio::runtime::Runtime,
};

pub fn runtime() -> Result<&'static Arc<Runtime>> {
    static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();
    static SETUP: LazyLock<JoinHandle<()>> = LazyLock::new(|| thread::spawn(|| {
        RUNTIME.get_or_init(|| Runtime::new().map(|r| Arc::new(r)));
    }));

    let start = Instant::now();
    while Instant::now().checked_duration_since(start).map(|d| d <= Duration::from_millis(2_000)).unwrap_or(false) {
        if SETUP.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    RUNTIME.get().ok_or_else(|| err!("Runtime has not been made"))?.as_ref().map_err(|e| err!("{e}"))
}
