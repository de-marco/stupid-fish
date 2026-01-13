use {
    std::{
        os::unix::net::SocketAddr,
        sync::LazyLock,
    },
    super::{MAP_KIND, Result},
    self::message::Message,
    lacol_rpc::{
        debts::nairud::Nairud,
        request::Request,
    },
    tokio::{
        net::UnixDatagram,
        sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
        task,
    },
};

pub mod message;
pub mod request;

mod c_api;

pub type Pid = u32;

pub (super) const UDS_STATUS_SERVER: &str = "uds-status";

const UNIX_DATAGRAM_BUF: usize = 2048;

static SENDER: LazyLock<UnboundedSender<Message>> = LazyLock::new(|| {
    let (sender, receiver) = mpsc::unbounded_channel();
    if let Ok(runtime) = super::runtime() {
        runtime.spawn(run_channel_server(receiver));
        runtime.spawn(start_uds_status_server(sender.clone()));
    }
    sender
});

pub (super) async fn start_server() -> Result<()> {
    let _ = &*SENDER;
    Ok(())
}

pub (in crate::hack) fn sender() -> &'static UnboundedSender<Message> {
    &*SENDER
}

async fn run_channel_server(mut receiver: UnboundedReceiver<Message>) {
    let mut clients = Vec::<SocketAddr>::with_capacity(3);
    let mut current_pid = None;
    while let Some(message) = receiver.recv().await {
        let mut send_data = async |data: Vec<_>| {
            let mut i = usize::MIN;
            while i < clients.len() {
                match async {
                    let stream = std::os::unix::net::UnixDatagram::unbound()?;
                    stream.connect_addr(&clients[i])?;
                    stream.set_nonblocking(true)?;
                    UnixDatagram::try_from(stream)?.send(&data).await
                }.await {
                    Ok(_) => i += 1,
                    Err(_) => drop(clients.swap_remove(i)),
                };
            }
        };
        match message {
            Message::NewClient(client_address) => clients.push(client_address),
            Message::NewProcess { id, exe } => {
                current_pid = Some(id);
                if let Ok(data) = Nairud::from_iter([Nairud::from(id), Nairud::from(exe)]).encode_as_vec() {
                    send_data(data).await;
                }
            },
            Message::ProcessFinished(pid) => if current_pid == Some(pid) {
                current_pid = None;
                if let Ok(data) = Nairud::None.encode_as_vec() {
                    send_data(data).await;
                }
            },
        };
    }
}

async fn start_uds_status_server(sender: UnboundedSender<Message>) -> Result<()> {
    let listener = std::os::unix::net::UnixDatagram::bind_addr(&super::make_socket_address(UDS_STATUS_SERVER)?)?;
    listener.set_nonblocking(true)?;
    let listener = UnixDatagram::try_from(listener)?;
    loop {
        let mut buf = [u8::MIN; UNIX_DATAGRAM_BUF];
        if let Ok((size, client_address)) = listener.recv_from(&mut buf).await {
            let sender = sender.clone();
            let data = buf[..size].to_vec();
            task::spawn(async move {
                if let Ok(Some(Ok(request))) = Nairud::decode(&mut &data[..], MAP_KIND).map(|n| n.map(|n| Request::try_from(n))) {
                    if let Ok(r) = self::request::Request::try_from(request.code()) {
                        //  Ignore error from sender.send()
                        match r {
                            self::request::Request::ReportNewProcess => if let Ok([id, exe]) = <[Nairud; 2]>::try_from(request.into_data()) {
                                if let (Ok(id), Ok(exe)) = (id.try_into(), exe.try_into()) {
                                    if sender.send(Message::NewProcess { id, exe }).is_err() {}
                                }
                            },
                            self::request::Request::WatchForProcesses => if sender.send(Message::NewClient(client_address.into())).is_err() {},
                        };
                    }
                }
                Result::Ok(())
            });
        }
    }
}
