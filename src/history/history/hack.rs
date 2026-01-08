extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        io::{BufReader, BufWriter, Read, Result, Write},
        os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixListener},
        },
        path::MAIN_SEPARATOR,
        process,
        sync::LazyLock,
        thread,
        time::SystemTime,
    },
    super::{History, HistoryItem, PersistenceMode},
    fake_log::{__err, __info},
    sj::{Array, Json},
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

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";
const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

pub (super) static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| Runtime::new().expect("Failed to make runtime"));

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    match Arc::clone(&history) {
        history => thread::spawn(move || start_provider_server(history)),
    };
    thread::spawn(move || start_manager_server(history));

    Ok(())
}

fn start_provider_server(history: Arc<History>) -> Result<()> {
    let raw_addr = format!("{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}provider", process::id());
    let listener = UnixListener::bind_addr(&SocketAddr::from_abstract_name(&raw_addr)?)?;
    __info!("-> {raw_addr}\n");
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                thread::spawn(move || {
                    let json = {
                        let history_impl = RUNTIME.block_on(history.0.lock());
                        Json::from_iter(history_impl.new_items.iter().map(|i| i.str().to_string()))
                    };
                    let mut stream = BufWriter::new(stream);
                    stream.write_all(&json.format_as_bytes()?)?;
                    stream.flush()
                });
            },
            Err(err) => __err!("{}", __!("Failed: {err}\n")),
        };
    }
}

fn start_manager_server(history: Arc<History>) -> Result<()> {
    let raw_addr = format!("{}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}manager", process::id());
    __info!("-> {raw_addr}\n");
    let listener = UnixListener::bind_addr(&SocketAddr::from_abstract_name(&raw_addr)?)?;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                thread::spawn(move || {
                    let stream = BufReader::new(stream);
                    let mut buf = Vec::with_capacity(1024);
                    stream.take(1024 * 1024).read_to_end(&mut buf)?;

                    let mut history_impl = RUNTIME.block_on(history.0.lock());
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
                });
            },
            Err(err) => __err!("{}", __!("Failed: {err}\n")),
        };
    }
}
