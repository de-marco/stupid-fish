extern crate alloc;

use {
    core::{
        ffi::{c_int, c_void},
        time::Duration,
    },
    alloc::sync::Arc,
    std::{
        sync::OnceLock,
        thread,
        time::Instant,
    },
    crate::hack::{MAP_KIND, Result},
    super::UNIX_DATAGRAM_BUF,
    fake_log::__err,
    lacol_rpc::{
        debts::nairud::Nairud,
        request::Request,
    },
    libc::size_t,
    sj::Json,
    tokio::{
        net::UnixDatagram,
        runtime::Runtime,
    },
};

type Callback = unsafe extern "C" fn(*const u8, size_t, *const c_void);

static RUNTIME: OnceLock<Result<Arc<Runtime>>> = OnceLock::new();

#[unsafe(no_mangle)]
extern "C" fn stupid_fish_start_process_watcher(pid: u32, f: Callback, user_data: *const c_void) -> c_int {
    thread::spawn(|| RUNTIME.get_or_init(|| Runtime::new().map(|r| Arc::new(r))));
    let start_time = Instant::now();
    while Instant::now().checked_duration_since(start_time).map(|d| d <= Duration::from_millis(100)).unwrap_or(false) {
        match RUNTIME.get() {
            None => thread::sleep(Duration::from_millis(10)),
            Some(Err(err)) => {
                __err!("{err}\n");
                break;
            },
            Some(Ok(runtime)) => {
                let user_data = user_data as usize;
                runtime.spawn(async move {
                    if let Result::<()>::Err(err) = async {
                        let stream = std::os::unix::net::UnixDatagram::unbound()?;
                        stream.connect_addr(&super::super::make_socket_address_with(pid, super::UDS_STATUS_SERVER)?)?;
                        stream.set_nonblocking(true)?;
                        let stream = UnixDatagram::try_from(stream)?;
                        stream.send(&Nairud::from(Request::new(super::request::Request::WatchForProcesses, ())).encode_as_vec()?).await?;
                        loop {
                            let mut buf = [u8::MIN; UNIX_DATAGRAM_BUF];
                            let size = stream.recv(&mut buf).await?;
                            let json = match Nairud::decode(&mut &buf[..size], MAP_KIND)? {
                                Some(Nairud::Array(array)) => Json::from_iter([
                                    Json::from(u32::try_from(&array[usize::MIN])?),
                                    Json::from(array[1].as_str()?),
                                ]),
                                Some(Nairud::None) => sj::array(),
                                other => return Err(err!("Unexpected data from server: {other:?}")),
                            };
                            let json = json.format_as_bytes()?;
                            unsafe {
                                f(json.as_ptr(), json.len(), user_data as *const c_void);
                            }
                        }
                    }.await {
                        __err!("{err}\n");
                    }
                });
                return 0;
            },
        };
    }
    -1
}
