use std::{array::TryFromSliceError, fs, path::PathBuf};

use eyre::{eyre, Context};
use spdlog::prelude::*;
use {argh::FromArgs, std::fmt::Debug};

mod stored_sec_path;

mod check;
mod deploy;
mod edit;
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
pub struct DeploySubCmd {
    #[argh(option, short = 's')]
    /// per hostkey encrypted dir
    storage: Option<String>,
}

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

        // Maybe clean first?
        let flake_root = if let Some(f) = &self.flake_root {
            PathBuf::from(f)
        } else {
            std::env::current_dir()?
        };

        // check flake root
        if !fs::read_dir(&flake_root)?.into_iter().any(|e| {
            e.is_ok_and(|ie| {
                ie.file_name()
                    .into_string()
                    .is_ok_and(|iie| iie.as_str() == "flake.nix")
            })
        }) {
            error!("please run app in flake root");
            return Err(eyre!(
                "`flake.nix` not found here, make sure run in flake toplevel."
            ));
        };

        trace!("{:#?}", profile);

        match self.app {
            SubCmd::Renc(RencSubCmd { all }) => {
                info!("start re-encrypt secrets");
                profile.renc(all, flake_root)
            }
            SubCmd::Deploy(DeploySubCmd { ref storage }) => {
                info!("deploying secrets");
                profile.deploy()
            }
            SubCmd::Edit(_) => todo!(),
            SubCmd::Check(_) => todo!(),
        }
    }
}
