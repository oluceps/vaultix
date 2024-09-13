use std::fs;

use spdlog::prelude::*;
use {argh::FromArgs, std::fmt::Debug};

mod check;
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
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCmd {
    Renc(RencSubCmd),
    Edit(EditSubCmd),
    Check(CheckSubCmd),
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
/// Check secret status
#[argh(subcommand, name = "check")]
pub struct CheckSubCmd {}

impl Args {
    /// Parse Command Args
    pub fn ayaya(&self) -> eyre::Result<()> {
        use super::profile::Profile;

        let profile: Profile = {
            let file = fs::read_to_string(&self.profile)?;
            toml::from_str(file.as_str())?
        };

        trace!("{:#?}", profile);

        match self.app {
            SubCmd::Renc(RencSubCmd { all }) => {
                info!("start re-encrypt secrets");
                profile.renc(all)
            }
            SubCmd::Edit(_) => todo!(),
            SubCmd::Check(_) => todo!(),
        }
    }
}