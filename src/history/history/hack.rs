extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::Result,
        thread,
    },
    super::History,
    self::server::Server,
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
    match Arc::clone(&history) {
        history => thread::spawn(move || Server::Provider.start(history)),
    };
    match Arc::clone(&history) {
        history => thread::spawn(move || Server::Manager.start(history)),
    };
    thread::spawn(move || Server::CdHistoryProvider.start(history));

    Ok(())
}
