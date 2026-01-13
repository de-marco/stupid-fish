use {
    core::ffi::{c_int, c_void},
    std::os::unix::net::UnixDatagram,
    crate::hack::{self, MAP_KIND, Result},
    super::{
        super::{make_random_socket_address, make_socket_address_with},
        UNIX_DATAGRAM_BUF,
    },
    fake_log::__err,
    lacol_rpc::{
        debts::nairud::Nairud,
        request::Request,
    },
    libc::size_t,
    sj::Json,
};

#[cfg(test)]
mod tests;

type Callback = unsafe extern "C" fn(*const u8, size_t, *const c_void) -> bool;

#[unsafe(no_mangle)]
extern "C" fn stupid_fish_start_process_watcher(pid: u32, f: Callback, user_data: *const c_void) -> c_int {
    match hack::c_api::runtime() {
        Err(err) => {
            __err!("{err}\n");
            -1
        },
        Ok(runtime) => {
            let user_data = user_data as usize;
            runtime.spawn_blocking(move || {
                if let Result::<()>::Err(err) = (|| {
                    let socket = UnixDatagram::bind_addr(&make_random_socket_address()?)?;
                    socket.send_to_addr(
                        &Nairud::from(Request::new(super::request::Request::WatchForProcesses, ())).encode_as_vec()?,
                        &make_socket_address_with(pid, super::UDS_STATUS_SERVER)?,
                    )?;
                    loop {
                        let mut buf = [u8::MIN; UNIX_DATAGRAM_BUF];
                        let (size, _) = socket.recv_from(&mut buf)?;
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
                            if f(json.as_ptr(), json.len(), user_data as *const c_void) == false {
                                return Ok(());
                            }
                        }
                    }
                })() {
                    __err!("{err}\n");
                }
            });
            0
        },
    }
}
