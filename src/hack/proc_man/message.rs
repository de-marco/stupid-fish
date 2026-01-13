use {
    std::os::unix::net::SocketAddr,
    super::Pid,
};

pub (in crate::hack) enum Message {
    NewClient(SocketAddr),
    RemoveClient(usize),
    NewProcess { id: Pid, exe: String },
    ProcessFinished(Pid),
}
