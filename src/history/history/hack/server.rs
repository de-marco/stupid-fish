extern crate alloc;

use {
    core::time::Duration,
    alloc::sync::Arc,
    std::{
        io::Result,
        sync::TryLockError,
        thread,
        time::{Instant, SystemTime, UNIX_EPOCH},
    },
    super::super::{History, HistoryItem, PersistenceMode},
    fake_log::__err,
    sj::{Array, Json},
    tokio::{
        io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
        net::UnixListener,
        task,
    },
};

const SJ_MAP_KIND: sj::MapKind = sj::MapKind::HashMap;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub (super) enum Server {
    HistoryProvider,
    HistoryManager,
    CdHistoryProvider,
    CdHistoryManager,
}

impl Server {

    pub const fn id(&self) -> &str {
        match self {
            Self::HistoryProvider => "history-provider",
            Self::HistoryManager => "history-manager",
            Self::CdHistoryProvider => "cd-history-provider",
            Self::CdHistoryManager => "cd-history-manager",
        }
    }

    pub async fn start(&self, history: Arc<History>) -> Result<()> {
        let bind = || crate::hack::bind(self.id());
        match self {
            Self::HistoryProvider => start_provider_server(bind()?, history).await,
            Self::HistoryManager => start_manager_server(bind()?, history).await,
            Self::CdHistoryProvider => start_cd_history_provider_server(bind()?).await,
            Self::CdHistoryManager => start_cd_history_manager_server(bind()?).await,
        }
    }

}

async fn start_provider_server(listener: UnixListener, history: Arc<History>) -> Result<()> {
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

async fn start_manager_server(listener: UnixListener, history: Arc<History>) -> Result<()> {
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
                    }).await?
                });
            },
            Err(err) => __err!("{}", __!("{err}\n")),
        };
    }
}

async fn start_cd_history_provider_server(listener: UnixListener) -> Result<()> {
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

async fn start_cd_history_manager_server(listener: UnixListener) -> Result<()> {
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                task::spawn(async move {
                    if let Err(err) = async {
                        let paths = {
                            macro_rules! limit { () => { 1024 * 1024 }}
                            let mut stream = BufReader::new(stream).take(limit!());
                            let mut data = Vec::with_capacity(limit!());
                            stream.read_to_end(&mut data).await?;
                            Array::try_from(sj::parse_bytes(data, SJ_MAP_KIND)?)?
                        };
                        task::spawn_blocking(move || {
                            let start = Instant::now();
                            loop {
                                match crate::builtins::cd::history::GLOBAL.try_write() {
                                    Ok(mut history) => {
                                        history.clear();
                                        for p in paths {
                                            history.add(String::try_from(p)?);
                                        }
                                        return Ok(());
                                    },
                                    Err(TryLockError::Poisoned(_)) => crate::builtins::cd::history::GLOBAL.clear_poison(),
                                    Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
                                };
                                if Instant::now().checked_duration_since(start).map(|d| d >= Duration::from_secs(1)).unwrap_or(true) {
                                    return Err(err!("Timed out waiting to write global history"));
                                }
                            }
                        }).await?
                    }.await {
                        __err!("{}", __!("{err}\n"));
                    }
                });
            },
            Err(err) => __err!("{}", __!("{err}\n")),
        };
    }
}
