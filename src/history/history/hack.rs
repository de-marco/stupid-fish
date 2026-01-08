extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::{Error, Result},
        os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixListener},
        },
        path::MAIN_SEPARATOR,
        process,
        sync::LazyLock,
        time::SystemTime,
    },
    super::{History, HistoryItem, PersistenceMode},
    fake_log::__info,
    sj::{Array, Json},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        runtime::Runtime,
    },
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

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";
const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

static RUNTIME: LazyLock<Result<Runtime>> = LazyLock::new(|| Runtime::new());

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    let runtime = RUNTIME.as_ref().map_err(|e| Error::from(e.kind()))?;

    runtime.spawn(start_provider_server(Arc::clone(&history)));
    runtime.spawn(start_manager_server(history));

    Ok(())
}

async fn start_provider_server(history: Arc<History>) -> Result<()> {
    let raw_addr = format!(
        "{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}provider", process::id(),
    );
    let addr = SocketAddr::from_abstract_name(&raw_addr)?;
    let listener = tokio::net::UnixListener::try_from(UnixListener::bind_addr(&addr)?)?;
    __info!("-> {raw_addr}\n");
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let history = Arc::clone(&history);
            if let Err(_) = async move {
                let json = {
                    let history_impl = history.0.try_lock().map_err(|_| err!())?;
                    Json::from_iter(history_impl.new_items.iter().map(|i| i.str().to_string()))
                };
                stream.write_all(&json.format_as_bytes()?).await?;
                stream.flush().await
            }.await {
                // Ignore it
            }
        }
    }
}

async fn start_manager_server(history: Arc<History>) -> Result<()> {
    let raw_addr = format!(
        "{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}manager", process::id(),
    );
    let addr = SocketAddr::from_abstract_name(&raw_addr)?;
    __info!("-> {raw_addr}\n");
    let listener = tokio::net::UnixListener::try_from(UnixListener::bind_addr(&addr)?)?;
    loop {
        if let Ok((stream, _)) = listener.accept().await {
            let history = Arc::clone(&history);
            if let Err(_) = async move {
                let mut buf = Vec::with_capacity(1024);
                stream.take(1024 * 1024).read_to_end(&mut buf).await?;

                let mut history_impl = history.0.try_lock().map_err(|_| err!())?;
                history_impl.clear();
                for item in Array::try_from(sj::parse_bytes(buf, SJ_MAP_KIND)?)? {
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
            }.await {
                // Ignore it
            }
        }
    }
}
