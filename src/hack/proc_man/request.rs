use {
    core::mem,
    std::io::Error,
    super::super::Result,
    lacol_rpc::request::Code,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub (in crate::hack) enum Request {
    ReportNewProcess,
    WatchForProcesses,
}

impl Request {

    const fn all() -> &'static [Self; mem::variant_count::<Self>()] {
        &[Self::ReportNewProcess, Self::WatchForProcesses]
    }

    const fn id(&self) -> u64 {
        *self as u64
    }

}

impl From<Request> for Code {

    fn from(request: Request) -> Self {
        request.id()
    }

}

impl TryFrom<Code> for Request {

    type Error = Error;

    fn try_from(code: Code) -> Result<Self> {
        for r in Self::all() {
            if r.id() == code {
                return Ok(*r);
            }
        }
        Err(err!("Unknown request: {code}"))
    }

}
