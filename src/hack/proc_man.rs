use {
    std::sync::LazyLock,
    super::Result,
    self::message::Message,
    lacol_rpc::debts::nairud::Nairud,
    tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    uds::{
        UnixSocketAddr,
        tokio::UnixSeqpacketListener,
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
    let mut clients = Vec::with_capacity(3);
    while let Some(message) = receiver.recv().await {
        // TODO
        match message {
            Message::NewClient(stream) => clients.push(stream),
            Message::ProcessFinished(_) => {
                let mut i = usize::MIN;
                while i < clients.len() {
                    macro_rules! go_up { () => { i += 1 }}
                    match Nairud::None.encode_as_vec() {
                        Ok(data) => if clients[i].send(&data).await.is_ok() {
                            go_up!();
                        } else {
                            clients.swap_remove(i);
                        },
                        Err(_) => go_up!(),
                    };
                }
            },
        };
    }
}

async fn start_uds_status_server(sender: UnboundedSender<Message>) -> Result<()> {
    let mut listener = UnixSeqpacketListener::bind_addr(&UnixSocketAddr::from_abstract(&super::form_address("uds-status"))?)?;
    loop {
        if let Ok((stream, _)) = listener.accept().await {
            if sender.send(Message::NewClient(stream)).is_err() {
                return Ok(());
            }
        }
    }
}
