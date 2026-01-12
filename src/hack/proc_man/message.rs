use {
    super::Pid,
    uds::tokio::UnixSeqpacketConn,
};

pub (super) enum Message {
    NewClient(UnixSeqpacketConn),
    ProcessFinished(Pid),
}
