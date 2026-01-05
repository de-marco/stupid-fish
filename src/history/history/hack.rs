extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::{Result, Write},
        os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixListener},
        },
        path::MAIN_SEPARATOR,
        process,
        thread,
    },
    super::History,
    sj::Json,
};

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    start_provider_server(Arc::clone(&history))?;
    Ok(())
}

fn start_provider_server(history: Arc<History>) -> Result<()> {
    const PROVIDER_SERVER: &str = "provider";

    thread::spawn(move || {
        let addr = SocketAddr::from_abstract_name(format!(
            "{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{PROVIDER_SERVER}", process::id(),
        ))?;
        let listener = UnixListener::bind_addr(&addr)?;
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                if let Ok(history_impl) = history.0.try_lock() {
                    if let Err(_) = (|| {
                        let json = Json::from_iter(history_impl.new_items.iter().map(|i| i.str().to_string()));
                        stream.write_all(&json.format_as_bytes()?)?;
                        stream.flush()
                    })() {
                        // Ignore it
                    }
                }
            }
        }
        Result::Ok(())
    });
    Ok(())
}
