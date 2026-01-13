use {
    core::ffi::c_int,
    std::{
        os::unix::net::UnixDatagram,
        sync::OnceLock,
        thread,
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
};

type Callback = unsafe extern "C" fn(*const u8, size_t);

static THREAD: OnceLock<()> = OnceLock::new();

#[unsafe(no_mangle)]
extern "C" fn stupid_fish_start_process_watcher(pid: u32, f: Callback) -> c_int {
    THREAD.get_or_init(move || drop(thread::spawn(move || {
        if let Err(err) = (|| -> Result<()> {
            let request = Nairud::from(Request::new(super::request::Request::WatchForProcesses, ())).encode_as_vec()?;

            let stream = UnixDatagram::unbound()?;
            stream.connect_addr(&super::super::make_socket_address_with(pid, super::UDS_STATUS_SERVER)?)?;
            stream.send(&request)?;

            loop {
                let mut buf = [u8::MIN; UNIX_DATAGRAM_BUF];
                let size = stream.recv(&mut buf)?;
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
                    f(json.as_ptr(), json.len());
                }
            }
        })() {
            __err!("{err}\n");
        }
    })));
    0
}
