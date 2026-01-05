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
        time::SystemTime,
    },
    super::{History, HistoryItem, PersistenceMode},
    sj::{Array, Json},
};

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";
const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    start_provider_server(Arc::clone(&history))?;
    start_manager_server(history)?;
    Ok(())
}

fn start_provider_server(history: Arc<History>) -> Result<()> {
    thread::spawn(move || {
        let addr = SocketAddr::from_abstract_name(format!(
            "{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}provider", process::id(),
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

fn start_manager_server(history: Arc<History>) -> Result<()> {
    thread::spawn(move || {
        let addr = SocketAddr::from_abstract_name(format!(
            "{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}manager", process::id(),
        ))?;
        let listener = UnixListener::bind_addr(&addr)?;
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                if let Ok(mut history_impl) = history.0.try_lock() {
                    if let Err(_) = (|| {
                        history_impl.clear();
                        for item in Array::try_from(sj::parse(&mut stream, SJ_MAP_KIND)?)? {
                            history_impl.add(
                                HistoryItem::new(
                                    String::try_from(item)?.into(), SystemTime::now(),
                                    PersistenceMode::Memory,
                                ),
                                false,
                                false,
                            );
                        }
                        Result::Ok(())
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
