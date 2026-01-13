#![cfg(test)]

use {
    core::mem,
    std::collections::HashSet,
    crate::hack::Result,
    super::Request,
};

#[test]
fn tests() -> Result<()> {
    assert_eq!(Request::all().into_iter().collect::<HashSet<_>>().len(), mem::variant_count::<Request>());

    for (index, item) in Request::all().into_iter().enumerate() {
        assert_eq!(usize::try_from(item.id()).unwrap(), index);
    }

    Ok(())
}
