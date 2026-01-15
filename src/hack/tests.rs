#![cfg(test)]

use super::{ALLOW_LOADING_HISTORY_FROM_FILES, Result};

#[test]
fn tests() -> Result<()> {
    assert_eq!(ALLOW_LOADING_HISTORY_FROM_FILES, false);
    Ok(())
}
