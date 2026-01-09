#![cfg(test)]

use {
    std::io::Result,
    super::History,
};

#[test]
fn tests() -> Result<()> {
    let mut hist = History::new(3);

    hist.add("aaa");
    hist.add("bbb");
    hist.add("ccc");
    assert_eq!(hist.len(), 3);

    // Adding existing → updates time, doesn't grow size
    hist.add("bbb");
    assert_eq!(hist.len(), 3);

    // Adding new → oldest ("a") should be removed
    hist.add("ddd");
    assert_eq!(hist.len(), 3);

    let recent: Vec<_> = hist.recents().map(|(_, p)| p).collect();
    assert_eq!(recent, vec!["ddd", "bbb", "ccc"]);

    Ok(())
}
