#![cfg(test)]

use {
    std::io::Result,
    super::History,
};

#[test]
fn tests() -> Result<()> {
    let mut hist = History::new(3);

    hist.add("a");
    hist.add("b");
    hist.add("c");
    assert_eq!(hist.len(), 3);

    // Adding existing → updates time, doesn't grow size
    hist.add("b");
    assert_eq!(hist.len(), 3);

    // Adding new → oldest ("a") should be removed
    hist.add("d");
    assert_eq!(hist.len(), 3);

    let recent: Vec<_> = hist.recents().map(|(p, _)| p).collect();
    assert_eq!(recent, vec!["d", "b", "c"]);

    Ok(())
}
