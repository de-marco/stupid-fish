extern crate alloc;

use {
    core::time::Duration,
    alloc::sync::Arc,
    std::{
        io::Result,
        os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixListener},
        },
        path::MAIN_SEPARATOR,
        process,
        sync::TryLockError,
        thread,
        time::{Instant, SystemTime, UNIX_EPOCH},
    },
    super::super::{History, HistoryItem, PersistenceMode},
    fake_log::{__err, __info},
    sj::{Array, Json},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
        task,
    },
};

const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";
const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub (super) enum Server {
    HistoryProvider,
    HistoryManager,
    CdHistoryProvider,
}

impl Server {

    pub const fn id(&self) -> &str {
        match self {
            Self::HistoryProvider => "history-provider",
            Self::HistoryManager => "history-manager",
            Self::CdHistoryProvider => "cd-history-provider",
        }
    }

    pub async fn start(&self, history: Arc<History>) -> Result<()> {
        let make_addr = || {
            let raw_addr = format!("{process_id}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{id}", process_id=process::id(), id=self.id());
            let result = SocketAddr::from_abstract_name(&raw_addr)?;
            __info!("-> {raw_addr}\n");
            Result::Ok(result)
        };
        match self {
            Self::HistoryProvider => start_provider_server(make_addr()?, history).await,
            Self::HistoryManager => start_manager_server(make_addr()?, history).await,
            Self::CdHistoryProvider => start_cd_history_provider_server(make_addr()?).await,
        }
    }

}

async fn start_provider_server(address: SocketAddr, history: Arc<History>) -> Result<()> {
    let listener = bind_addr(&address)?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                task::spawn(async move {
                    let json = task::spawn_blocking(move || {
                        let history_impl = history.imp();
                        Json::from_iter(history_impl.new_items.iter().map(|i| i.str().to_string()))
                    }).await?;
                    let mut stream = BufWriter::new(stream);
                    stream.write_all(&json.format_as_bytes()?).await?;
                    stream.flush().await
                });
            },
            Err(err) => __err!("{}", __!("{err}\n")),
        };
    }
}

async fn start_manager_server(address: SocketAddr, history: Arc<History>) -> Result<()> {
    let listener = bind_addr(&address)?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let history = Arc::clone(&history);
                task::spawn(async move {
                    let stream = BufReader::new(stream);
                    let mut buf = Vec::with_capacity(1024);
                    stream.take(1024 * 1024).read_to_end(&mut buf).await?;
                    task::spawn_blocking(move || {
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
                    }).await??;
                    Result::Ok(())
                });
            },
            Err(err) => __err!("{}", __!("{err}\n")),
        };
    }
}

async fn start_cd_history_provider_server(address: SocketAddr) -> Result<()> {
    let listener = bind_addr(&address)?;
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                task::spawn(async move {
                    if let Err(err) = async {
                        let json = task::spawn_blocking(|| {
                            let start = Instant::now();
                            loop {
                                match crate::builtins::cd::history::GLOBAL.try_read() {
                                    Ok(history) => return Ok(Json::from_iter(history.recents().map(|(time, path)| Json::from_iter([
                                        Json::from(time.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(u64::MIN)),
                                        Json::from(path.to_string()),
                                    ])))),
                                    Err(TryLockError::Poisoned(_)) => crate::builtins::cd::history::GLOBAL.clear_poison(),
                                    Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
                                };
                                if Instant::now().checked_duration_since(start).map(|d| d >= Duration::from_secs(1)).unwrap_or(true) {
                                    return Err(err!("Timed out waiting to read global history"));
                                }
                            }
                        }).await??;
                        let mut stream = BufWriter::new(stream);
                        stream.write_all(&json.format_as_bytes()?).await?;
                        stream.flush().await
                    }.await {
                        __err!("{}", __!("{err}\n"));
                    }
                });
            },
            Err(err) => __err!("{}", __!("{err}\n")),
        };
    }
}

fn bind_addr(address: &SocketAddr) -> Result<tokio::net::UnixListener> {
    let listener = UnixListener::bind_addr(&address)?;
    listener.set_nonblocking(true)?;
    listener.try_into()
}
