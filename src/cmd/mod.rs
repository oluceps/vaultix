use std::{fs, path::PathBuf};

use eyre::{eyre, Context, ContextCompat};
use spdlog::prelude::*;
use {argh::FromArgs, std::fmt::Debug};

mod check;
mod deploy;
mod edit;
mod renc;

#[derive(FromArgs, PartialEq, Debug)]
/// Vaultix cli | Secret manager for NixOS
pub struct Args {
    #[argh(subcommand)]
    app: SubCmd,
    #[argh(option, short = 'p')]
    /// secret profile
    profile: Option<String>,
    #[argh(option, short = 'f')]
    /// toplevel of flake repository
    flake_root: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCmd {
    Renc(RencSubCmd),
    Edit(EditSubCmd),
    Check(CheckSubCmd),
    Deploy(DeploySubCmd),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Re-encrypt changed files
#[argh(subcommand, name = "renc")]
pub struct RencSubCmd {
    #[argh(option, short = 'i')]
    /// identity for decrypt secret
    identity: String,
    #[argh(option, short = 'c')]
    /// identity for decrypt secret
    cache: String,
}

#[derive(FromArgs, PartialEq, Debug, Clone)]
/// Edit encrypted file
#[argh(subcommand, name = "edit")]
pub struct EditSubCmd {
    #[argh(positional)]
    /// file to edit
    file: String,
    #[argh(option, short = 'i')]
    /// identity for decrypt secret
    identity: Option<String>,
    #[argh(option, short = 'r')]
    /// recipients for encrypt secrets
    recipients: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Decrypt and deploy cipher credentials
#[argh(subcommand, name = "deploy")]
pub struct DeploySubCmd {}

#[derive(FromArgs, PartialEq, Debug)]
/// Check secret status
#[argh(subcommand, name = "check")]
pub struct CheckSubCmd {}

impl Args {
    /// Parse Command Args
    pub fn ayaya(&self) -> eyre::Result<()> {
        use super::profile::Profile;

        let profile = || -> eyre::Result<Profile> {
            let file = self
                .profile
                .clone()
                .wrap_err_with(|| eyre!("this cmd requires provide profile"))
                .and_then(|p| fs::read_to_string(p).wrap_err_with(|| eyre!("read file error")))
                .wrap_err_with(|| eyre::eyre!("read profile error"))
                .wrap_err("arg `profile` not found")?;
            serde_json::from_str(file.as_str()).wrap_err_with(|| eyre::eyre!("parse profile fail"))
        };

        let flake_root = if let Some(f) = &self.flake_root {
            PathBuf::from(f)
        } else {
            std::env::current_dir()?
        };

        match &self.app {
            SubCmd::Renc(RencSubCmd { identity, cache }) => {
                debug!("start re-encrypt secrets");
                let profile = profile()?;
                profile.renc(flake_root, identity.clone(), cache.into())
            }
            SubCmd::Deploy(DeploySubCmd {}) => {
                info!("deploying secrets");
                let profile = profile()?;
                profile.deploy()
            }
            SubCmd::Edit(e) => {
                info!("editing secrets");
                edit::edit(e.clone())
            }
            SubCmd::Check(_) => {
                info!("start checking");
                let profile = profile()?;
                profile.check()?;
                info!("check complete");
                Ok(())
            }
        }
    }
}
