#![cfg(test)]

use {
    std::io::Result,
    super::History,
};

#[test]
fn tests() -> Result<()> {
    const MAX_SIZE: usize = 3;

    let mut hist = History::new(MAX_SIZE);

    macro_rules! assert_max_size { () => {
        assert_eq!(hist.by_time.len(), MAX_SIZE);
        assert_eq!(hist.by_path.len(), MAX_SIZE);
    }}

    hist.add("aaa");
    hist.add("bbb");
    hist.add("ccc");
    assert_max_size!();

    // Adding existing → updates time, doesn't grow size
    hist.add("bbb");
    assert_max_size!();

    // Adding new → oldest ("aaa") should be removed
    hist.add("ddd");
    assert_max_size!();

    let recent: Vec<_> = hist.recents().map(|(_, p)| p).collect();
    assert_eq!(recent, vec!["ddd", "bbb", "ccc"]);

    Ok(())
}
