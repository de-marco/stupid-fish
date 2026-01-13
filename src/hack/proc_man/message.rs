use {
    super::Pid,
    uds::tokio::UnixSeqpacketConn,
};

pub (in crate::hack) enum Message {
    NewClient(UnixSeqpacketConn),
    NewProcess { id: u32, exe: String },
    ProcessFinished(Pid),
}
