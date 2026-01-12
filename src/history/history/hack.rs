extern crate alloc;

use {
    alloc::sync::Arc,
    std::io::Result,
    super::History,
    self::server::Server,
};

mod server;

pub (super) fn start_servers(history: Arc<History>) -> Result<()> {
    let runtime = crate::hack::runtime()?;
    runtime.spawn(Server::HistoryProvider.start(Arc::clone(&history)));
    runtime.spawn(Server::HistoryManager.start(Arc::clone(&history)));
    runtime.spawn(Server::CdHistoryProvider.start(history));

    Ok(())
}
