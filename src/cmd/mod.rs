use std::{fs, path::PathBuf};

use eyre::{eyre, Context, ContextCompat};
use log::info;
use renc::CompleteProfile;
use {argh::FromArgs, std::fmt::Debug};

mod check;
mod deploy;
mod edit;
pub mod renc;

#[derive(FromArgs, PartialEq, Debug)]
/// Vaultix cli | Secret manager for NixOS
pub struct Args {
    #[argh(subcommand)]
    app: SubCmd,
    #[argh(option, short = 'p')]
    /// secret profile
    profile: Vec<String>,
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
    recipient: Vec<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Decrypt and deploy cipher credentials
#[argh(subcommand, name = "deploy")]
pub struct DeploySubCmd {
    #[argh(switch, short = 'e')]
    /// deploy before users init
    early: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Check secret status
#[argh(subcommand, name = "check")]
pub struct CheckSubCmd {}

impl Args {
    /// Parse Command Args
    pub fn ayaya(&self) -> eyre::Result<()> {
        use super::profile::Profile;

        let profile = || -> eyre::Result<Vec<Profile>> {
            self.profile
                .iter()
                .map(|p| {
                    fs::read_to_string(p)
                        .wrap_err_with(|| eyre!("read file error"))
                        .and_then(|f| {
                            serde_json::from_str(f.as_str())
                                .wrap_err_with(|| eyre::eyre!("parse profile fail"))
                        })
                })
                .collect()
        };

        let flake_root = if let Some(f) = &self.flake_root {
            PathBuf::from(f)
        } else {
            std::env::current_dir()?
        };

        match &self.app {
            SubCmd::Renc(RencSubCmd { identity, cache }) => {
                info!("start re-encrypt secrets");
                let profile = profile()?;
                CompleteProfile::from_iter(&profile).renc(
                    flake_root,
                    identity.clone(),
                    cache.into(),
                )
            }
            SubCmd::Deploy(DeploySubCmd { early }) => {
                info!("deploying secrets");
                let profile = profile()?;
                profile
                    .first()
                    .wrap_err_with(|| eyre!("deploy must provide one single profile"))?
                    .deploy(*early)
            }
            SubCmd::Edit(e) => {
                info!("editing secrets");
                edit::edit(e.clone())
            }
            SubCmd::Check(_) => {
                info!("start checking");
                let profile = profile()?;
                CompleteProfile::from_iter(&profile).check()?;
                info!("check complete");
                Ok(())
            }
        }
    }
}
