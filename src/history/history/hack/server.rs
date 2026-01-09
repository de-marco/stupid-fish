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
        thread,
        time::{SystemTime, UNIX_EPOCH},
    },
    super::super::{History, HistoryItem, PersistenceMode},
    fake_log::{__err, __info},
    sj::{Array, Json},
};

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";
const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub (super) enum Server {
    Provider,
    Manager,
    CdHistoryProvider,
}

impl Server {

    pub const fn id(&self) -> &str {
        match self {
            Self::Provider => "provider",
            Self::Manager => "manager",
            Self::CdHistoryProvider => "cd-history-provider",
        }
    }

    pub fn start(&self, history: Arc<History>) -> Result<()> {
        let make_addr = || {
            let raw_addr = format!("{process_id}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{id}", process_id=process::id(), id=self.id());
            let result = SocketAddr::from_abstract_name(&raw_addr)?;
            __info!("-> {raw_addr}\n");
            Result::Ok(result)
        };
        match self {
            Self::Provider => start_provider_server(make_addr()?, history),
            Self::Manager => start_manager_server(make_addr()?, history),
            Self::CdHistoryProvider => start_cd_history_provider_server(make_addr()?),
        }
    }

}

fn start_provider_server(address: SocketAddr, history: Arc<History>) -> Result<()> {
    let listener = UnixListener::bind_addr(&address)?;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                thread::spawn(move || {
                    let json = {
                        let history_impl = history.imp();
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

fn start_manager_server(address: SocketAddr, history: Arc<History>) -> Result<()> {
    let listener = UnixListener::bind_addr(&address)?;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                thread::spawn(move || {
                    let stream = BufReader::new(stream);
                    let mut buf = Vec::with_capacity(1024);
                    stream.take(1024 * 1024).read_to_end(&mut buf)?;

                    let mut history_impl = history.imp();
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

fn start_cd_history_provider_server(address: SocketAddr) -> Result<()> {
    let listener = UnixListener::bind_addr(&address)?;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                thread::spawn(move || {
                    let json = {
                        let history = crate::builtins::cd::history::History::GLOBAL;
                        let history = history.try_read().map_err(|e| err!("{e}"))?;
                        Json::from_iter(history.recents().map(|(time, path)| Json::from_iter([
                            Json::from(time.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(u64::MIN)),
                            Json::from(path.to_string()),
                        ])))
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
