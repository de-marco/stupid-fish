use std::os::unix::net::SocketAddr;

#[derive(Debug, Clone)]
pub (super) enum Message {
    NewClient(SocketAddr),
    RemoveClient(u64),
    Cd(String),
}
