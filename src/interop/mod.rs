use std::{
    path::Path,
    process::{Command, Output},
};

use eyre::{eyre, Result};

pub fn add_to_store<P: AsRef<Path>>(p: P) -> Result<Output> {
    Command::new("nix")
        .arg("store")
        .arg("add-path")
        .arg(p.as_ref())
        .output()
        .map_err(|i| eyre!("nix cmd run failed {}", i))
}
