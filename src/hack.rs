extern crate alloc;

use {
    core::sync::atomic::{AtomicU64, Ordering},
    alloc::sync::Arc,
    std::{
        env,
        io::Write,
        os::{
            linux::net::SocketAddrExt,
            unix::net::{SocketAddr, UnixDatagram, UnixStream},
        },
        path::MAIN_SEPARATOR,
        process,
        sync::OnceLock,
        thread,
    },
    fake_log::__err,
    lacol_rpc::{
        debts::nairud::{MapKind, Nairud},
        request::Request,
    },
    tokio::{
        net::UnixListener,
        runtime::Runtime,
    },
};

pub mod c_api;

mod proc_man;

pub type Result<T> = std::io::Result<T>;

const MAP_KIND: MapKind = MapKind::HashMap;

static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();

pub fn setup() {
    thread::spawn(|| RUNTIME.get_or_init(|| {
        let runtime = Runtime::new().map(|r| Arc::new(r));
        if let Ok(runtime) = &runtime {
            runtime.spawn(proc_man::start_server());
        }
        runtime
    }));

    crate::builtins::cd::history::setup();

    connect_boss();
}

pub fn runtime() -> Result<&'static Arc<Runtime>> {
    RUNTIME.get().ok_or_else(|| err!("Runtime has not been made"))?.as_ref().map_err(|e| err!("{e}"))
}

pub fn bind<S>(id: S) -> Result<UnixListener> where S: AsRef<str> {
    let listener = std::os::unix::net::UnixListener::bind_addr(&make_socket_address(id)?)?;
    listener.set_nonblocking(true)?;
    listener.try_into()
}

pub fn make_socket_address<S>(id: S) -> Result<SocketAddr> where S: AsRef<str> {
    SocketAddr::from_abstract_name(form_address(id))
}

fn form_address<S>(id: S) -> String where S: AsRef<str> {
    form_address_with(process::id(), id)
}

pub fn make_socket_address_with<S>(process_id: u32, id: S) -> Result<SocketAddr> where S: AsRef<str> {
    SocketAddr::from_abstract_name(form_address_with(process_id, id))
}

fn form_address_with<S>(process_id: u32, id: S) -> String where S: AsRef<str> {
    const ADDRESS_PREFIX: &str = "57b8ce61-d64293cc-da5ed9e2-d8458614";

    format!("{process_id}{MAIN_SEPARATOR}{ADDRESS_PREFIX}{MAIN_SEPARATOR}{id}", id=id.as_ref())
}

pub fn make_random_socket_address() -> Result<SocketAddr> {
    static ID: AtomicU64 = AtomicU64::new(u64::MIN);
    static ATOMIC_ORDERING: Ordering = Ordering::Relaxed;

    SocketAddr::from_abstract_name(form_address(format!("auto/{}", ID.fetch_add(1, ATOMIC_ORDERING))))
}

pub fn report_new_process_to_host_process<S>(cmd: S) where S: AsRef<str> {
    //  We're forked here, do NOT use static variables
    if let Err(err) = (|| {
        if let Some(cmd) = cmd.as_ref().split_whitespace().next().map(|s| s.to_string()) {
            if cmd.is_empty() == false {
                let request = Nairud::from_iter([Nairud::from(process::id()), Nairud::from(cmd)]);
                let request = Nairud::from(Request::new(proc_man::request::Request::ReportNewProcess, request)).encode_as_vec()?;

                let socket = UnixDatagram::unbound()?;
                socket.send_to_addr(&request, &{
                    let host_pid = unsafe { libc::getppid() }.try_into().map_err(|_| err!())?;
                    make_socket_address_with(host_pid, proc_man::UDS_STATUS_SERVER)?
                })?;
            }
        }
        Result::Ok(())
    })() {
        __err!("{err}\n");
    }
}

pub fn report_process_finished(id: proc_man::Pid) {
    if let Err(err) = proc_man::sender().send(proc_man::message::Message::ProcessFinished(id)) {
        __err!("{err}\n");
    }
}

fn connect_boss() {
    const ENV_STUPID_FISH_BOSS: &str = "STUPID_FISH_BOSS";

    thread::spawn(move || {
        match env::var(ENV_STUPID_FISH_BOSS) {
            Ok(address) => {
                unsafe {
                    env::remove_var(ENV_STUPID_FISH_BOSS);
                }
                let mut stream = UnixStream::connect_addr(&SocketAddr::from_abstract_name(address.trim())?)?;
                stream.write_all(&process::id().to_le_bytes())?;
                stream.flush()?;
            },
            Err(err) => __err!("Failed getting {ENV_STUPID_FISH_BOSS:?}: {err}\n"),
        };
        Result::Ok(())
    });
}
