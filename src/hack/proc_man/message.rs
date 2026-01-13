use {
    super::Pid,
    uds::tokio::UnixSeqpacketConn,
};

pub (in crate::hack) enum Message {
    NewClient(UnixSeqpacketConn),
    ProcessFinished(Pid),
}
