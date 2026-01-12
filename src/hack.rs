use {
    std::{
        sync::LazyLock,
        thread::{self, ThreadId},
    },
    fake_log::__info,
};

pub static MAIN_THREAD_ID: LazyLock<ThreadId> = LazyLock::new(|| thread::current().id());

pub fn setup() {
    __info!("Main thread ID: {:?}\n", *MAIN_THREAD_ID);
    crate::builtins::cd::history::setup();
}
