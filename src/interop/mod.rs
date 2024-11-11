use std::{
    fmt::Display,
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

macro_rules! eval_nix_command {
    ($host:expr, $key:expr, $result_type:ty) => {{
        let cmd_output = Command::new("nix")
            .arg("eval")
            .arg(format!(
                ".#nixosConfigurations.{}.config.vaultix.settings.{}",
                $host, $key
            ))
            .output()
            .map_err(|e| eyre::eyre!("nix cmd run failed: {}", e))?
            .stdout;

        let res = serde_json::from_slice::<$result_type>(&cmd_output)?;
        Ok(res)
    }};
}

pub fn eval_extra_recipient<H: AsRef<str> + Display>(host: H) -> Result<Vec<String>> {
    eval_nix_command!(host.as_ref(), "extraRecipients", Vec<String>)
}

pub fn eval_host_pubkey<H: AsRef<str> + Display>(host: H) -> Result<String> {
    eval_nix_command!(host.as_ref(), "hostPubkey", String)
}
