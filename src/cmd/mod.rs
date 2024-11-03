use std::{fs, path::PathBuf};

use eyre::Context;
use spdlog::prelude::*;
use {argh::FromArgs, std::fmt::Debug};

mod check;
mod deploy;
// mod edit;
mod renc;

#[derive(FromArgs, PartialEq, Debug)]
/// Vaultix cli | Secret manager for NixOS
pub struct Args {
    #[argh(subcommand)]
    app: SubCmd,
    #[argh(positional)]
    /// toml secret profile
    profile: String,
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
    #[argh(switch, short = 'a')]
    /// rekey all
    all: bool,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Edit encrypted file
#[argh(subcommand, name = "edit")]
pub struct EditSubCmd {
    #[argh(positional)]
    /// file to edit
    file: String,
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

        let profile: Profile = {
            let file = fs::read_to_string(&self.profile).wrap_err("arg `profile` not found")?;
            toml::from_str(file.as_str())?
        };

        let flake_root = if let Some(f) = &self.flake_root {
            PathBuf::from(f)
        } else {
            std::env::current_dir()?
        };

        trace!("{:#?}", profile);

        match self.app {
            SubCmd::Renc(RencSubCmd { all }) => {
                info!("start re-encrypt secrets");
                profile.renc(all, flake_root)
            }
            SubCmd::Deploy(DeploySubCmd {}) => {
                info!("deploying secrets");
                profile.deploy()
            }
            SubCmd::Edit(_) => todo!("you can directly use rage."),
            SubCmd::Check(_) => {
                info!("start checking");
                profile.check()?;
                info!("check complete");
                Ok(())
            }
        }
    }
}
