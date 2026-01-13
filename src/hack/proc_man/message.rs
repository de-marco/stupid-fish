use {
    super::Pid,
    uds::tokio::UnixSeqpacketConn,
};

pub (in crate::hack) enum Message {
    NewClient(UnixSeqpacketConn),
    NewProcess { id: Pid, exe: String },
    ProcessFinished(Pid),
}
