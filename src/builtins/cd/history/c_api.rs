use {
    core::{
        ffi::{c_int, c_void},
        str,
    },
    std::os::unix::net::UnixDatagram,
    crate::hack::{self, Result, make_random_socket_address, make_socket_address_with},
    fake_log::__err,
    libc::size_t,
};

type Callback = unsafe extern "C" fn(*const u8, size_t, *const c_void) -> bool;

#[unsafe(no_mangle)]
extern "C" fn stupid_fish_start_cd_history_status(pid: u32, f: Callback, user_data: *const c_void) -> c_int {
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
                    socket.send_to_addr(&[u8::MIN], &make_socket_address_with(pid, super::STATUS_SERVER_ADDRESS)?)?;
                    loop {
                        let mut buf = [u8::MIN; 2048];
                        let (size, _) = socket.recv_from(&mut buf)?;
                        match str::from_utf8(&buf[..size]) {
                            Ok(_) => unsafe {
                                if f(buf[..size].as_ptr(), size, user_data as *const c_void) == false {
                                    return Ok(());
                                }
                            },
                            Err(_) => return Err(err!("Unexpected data from server")),
                        };
                    }
                })() {
                    __err!("{err}\n");
                }
            });
            0
        },
    }
}
