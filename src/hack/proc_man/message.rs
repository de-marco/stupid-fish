use {
    std::os::unix::net::SocketAddr,
    super::Pid,
};

pub (in crate::hack) enum Message {
    NewClient(SocketAddr),
    RemoveClient(u64),
    NewProcess { id: Pid, exe: String },
    ProcessFinished(Pid),
}
