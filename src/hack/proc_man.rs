extern crate alloc;

use {
    alloc::sync::Arc,
    std::{
        collections::HashMap,
        os::unix::net::{SocketAddr, UnixDatagram},
        sync::LazyLock,
    },
    super::{MAP_KIND, Result},
    self::message::Message,
    fake_log::__err,
    lacol_rpc::{
        debts::nairud::Nairud,
        request::Request,
    },
    tokio::{
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
        runtime.spawn(run_channel_server(sender.clone(), receiver));
        match sender.clone() {
            sender => runtime.spawn_blocking(move || start_uds_status_server(sender)),
        };
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

async fn run_channel_server(sender: UnboundedSender<Message>, mut receiver: UnboundedReceiver<Message>) {
    let mut client_id = u64::MIN;
    let mut clients = HashMap::<u64, SocketAddr>::with_capacity(3);
    let mut current_pid = None;
    while let Some(message) = receiver.recv().await {
        macro_rules! send_data { ($data: ident) => {
            let data = Arc::new($data);
            for (client_id, address) in &clients {
                let (data, client_id, address, sender) = (Arc::clone(&data), *client_id, address.clone(), sender.clone());
                task::spawn_blocking(move || {
                    if let Err(err) = (|| {
                        let socket = UnixDatagram::unbound()?;
                        socket.send_to_addr(&data, &address)
                    })() {
                        __err!("{err}\n");
                        if sender.send(Message::RemoveClient(client_id)).is_err() {}
                    }
                });
            }
        }}
        match message {
            Message::NewClient(client_address) => {
                clients.insert(client_id, client_address);
                client_id += 1;
            },
            Message::RemoveClient(id) => drop(clients.remove(&id)),
            Message::NewProcess { id, exe } => {
                current_pid = Some(id);
                if let Ok(data) = Nairud::from_iter([Nairud::from(id), Nairud::from(exe)]).encode_as_vec() {
                    send_data!(data);
                }
            },
            Message::ProcessFinished(pid) => if current_pid == Some(pid) {
                current_pid = None;
                if let Ok(data) = Nairud::None.encode_as_vec() {
                    send_data!(data);
                }
            },
        };
    }
}

fn start_uds_status_server(sender: UnboundedSender<Message>) -> Result<()> {
    let socket = UnixDatagram::bind_addr(&super::make_socket_address(UDS_STATUS_SERVER)?)?;
    loop {
        let mut buf = [u8::MIN; UNIX_DATAGRAM_BUF];
        if let Ok((size, client_address)) = socket.recv_from(&mut buf) {
            let sender = sender.clone();
            let data = buf[..size].to_vec();
            super::runtime()?.spawn_blocking(move || {
                if let Err(err) = (|| {
                    let request = Request::try_from(Nairud::decode(&mut &data[..], MAP_KIND)?.ok_or_else(|| err!("No data"))?)?;
                    //  Ignore error from sender.send()
                    match self::request::Request::try_from(request.code())? {
                        self::request::Request::ReportNewProcess => if let Ok([id, exe]) = <[Nairud; 2]>::try_from(request.into_data()) {
                            if let (Ok(id), Ok(exe)) = (id.try_into(), exe.try_into()) {
                                if sender.send(Message::NewProcess { id, exe }).is_err() {}
                            }
                        },
                        self::request::Request::WatchForProcesses => if sender.send(Message::NewClient(client_address.into())).is_err() {},
                    };
                    Result::Ok(())
                })() {
                    __err!("{err}\n");
                }
            });
        }
    }
}
