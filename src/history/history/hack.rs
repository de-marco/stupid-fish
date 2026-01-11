extern crate alloc;

use {
    core::mem,
    alloc::sync::Arc,
    std::{
        io::Result,
        sync::OnceLock,
        thread,
    },
    super::History,
    self::server::Server,
    fake_log::__err,
    tokio::runtime::Runtime,
};

/// # Wrapper for format!(), which prefixes your optional message with: module_path!(), line!()
macro_rules! __ {
    ($($arg: tt)+) => {
        format!("[{module_path}-{line}] {msg}", module_path=module_path!(), line=line!(), msg=format!($($arg)+))
    };
    () => {
        __!("(internal error)")
    };
}

/// # Makes new std::io::Error
macro_rules! err {
    ($kind: path, $($arg: tt)+) => { std::io::Error::new($kind, __!($($arg)+)) };
    ($($arg: tt)+) => { err!(std::io::ErrorKind::Other, $($arg)+) };
    () => { std::io::Error::new(std::io::ErrorKind::Other, __!()) };
}

mod server;

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    static RUNTIME_THREAD: OnceLock<()> = OnceLock::new();

    RUNTIME_THREAD.get_or_init(move || {
        thread::spawn(move || {
            match Runtime::new() {
                Ok(runtime) => {
                    runtime.spawn(Server::HistoryProvider.start(Arc::clone(&history)));
                    runtime.spawn(Server::HistoryManager.start(Arc::clone(&history)));
                    runtime.spawn(Server::CdHistoryProvider.start(history));
                    mem::forget(runtime);
                },
                Err(err) => __err!("Failed making new runtime: {err}\n"),
            };
        });
    });

    Ok(())
}
