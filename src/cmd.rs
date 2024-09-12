use spdlog::prelude::*;
use {argh::FromArgs, std::fmt::Debug};

#[derive(FromArgs, PartialEq, Debug)]
/// Vaultix cli | Secret manager for NixOS
pub struct Args {
    #[argh(subcommand)]
    app: SubCmd,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum SubCmd {
    Renc(RencSubCmd),
    Edit(EditSubCmd),
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

impl Args {
    /// Parse Command Args
    pub fn ayaya(&self) -> eyre::Result<()> {
        match self.app {
            SubCmd::Renc(_) => {
                info!("command start");
                Ok(())
            }
            SubCmd::Edit(_) => todo!(),
        }
    }
}
