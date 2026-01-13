use {
    std::sync::LazyLock,
    super::{MAP_KIND, Result},
    self::message::Message,
    lacol_rpc::{
        debts::nairud::Nairud,
        request::Request,
    },
    tokio::{
        sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
        task,
    },
    uds::{
        UnixSocketAddr,
        tokio::{UnixSeqpacketConn, UnixSeqpacketListener},
    },
};

mod message;
mod request;

type Pid = ();

static FINISHED_SENDER: LazyLock<UnboundedSender<Message>> = LazyLock::new(|| {
    let (sender, receiver) = mpsc::unbounded_channel();
    if let Ok(runtime) = super::runtime() {
        runtime.spawn(run_finished_server(receiver));
        runtime.spawn(start_uds_status_server(sender.clone()));
    }
    sender
});

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub (super) enum Task {
    //  Use Unix Domain socket to receive startup signals from forked processes
    Startup,
    //  Use mpsc to receive finished signals of forked processes within current process.
    Finished,
}

impl Task {

    const fn id(&self) -> &str {
        match self {
            Self::Startup => "startup",
            Self::Finished => "finished",
        }
    }

    pub async fn start_server(&self) -> Result<()> {
        match self {
            Self::Startup => todo!(),
            Self::Finished => start_finished_server().await,
        }
    }

}

async fn start_finished_server() -> Result<()> {
    let _ = &*FINISHED_SENDER;
    Ok(())
}

pub (super) fn finished_sender() -> &'static UnboundedSender<Message> {
    &*FINISHED_SENDER
}

async fn run_finished_server(mut receiver: UnboundedReceiver<Message>) {
    let mut clients = Vec::<UnixSeqpacketConn>::with_capacity(3);
    while let Some(message) = receiver.recv().await {
        let mut send_data = async |data: Vec<_>| {
            let mut i = usize::MIN;
            while i < clients.len() {
                if clients[i].send(&data).await.is_ok() {
                    i += 1;
                } else {
                    clients.swap_remove(i);
                }
            }
        };
        match message {
            Message::NewClient(stream) => clients.push(stream),
            Message::NewProcess { id, exe } => if let Ok(data) = Nairud::from_iter([Nairud::from(id), Nairud::from(exe)]).encode_as_vec() {
                send_data(data).await;
            },
            Message::ProcessFinished(_) => if let Ok(data) = Nairud::None.encode_as_vec() {
                send_data(data).await;
            },
        };
    }
}

async fn start_uds_status_server(sender: UnboundedSender<Message>) -> Result<()> {
    let mut listener = UnixSeqpacketListener::bind_addr(&UnixSocketAddr::from_abstract(&super::form_address("uds-status"))?)?;
    loop {
        if let Ok((mut stream, _)) = listener.accept().await {
            let sender = sender.clone();
            task::spawn(async move {
                let mut buf = [u8::MIN; 256];
                let read = stream.recv(&mut buf).await?;
                if let Ok(Some(Ok(request))) = Nairud::decode(&mut &buf[..read], MAP_KIND).map(|n| n.map(|n| Request::try_from(n))) {
                    if let Ok(r) = self::request::Request::try_from(request.code()) {
                        //  Ignore error from sender.send()
                        match r {
                            self::request::Request::ReportNewProcess => if let Ok([id, exe]) = <[Nairud; 2]>::try_from(request.into_data()) {
                                if let (Ok(id), Ok(exe)) = (id.try_into(), exe.try_into()) {
                                    if sender.send(Message::NewProcess { id, exe }).is_err() {}
                                }
                            },
                            self::request::Request::WatchForProcesses => if sender.send(Message::NewClient(stream)).is_err() {},
                        };
                    }
                }
                Result::Ok(())
            });
        }
    }
}
