use eyre::{eyre, ContextCompat, Result};
use spdlog::{error, info};
use std::{fs, path::PathBuf};

use crate::helper::stored::Renc;
use crate::interop::add_to_store;
use crate::profile::{MasterIdentity, Profile};

use crate::helper::parse_identity::ParsedIdentity;
impl Profile {
    pub fn get_key_pair_iter<'a>(&'a self) -> impl Iterator<Item = Result<ParsedIdentity>> + 'a {
        self.settings
            .master_identities
            .iter()
            .map(MasterIdentity::parse)
    }

    /**
    read secret metadata from profile

    First decrypt `./secrets/every` with masterIdentity's privkey,
    Then compare hash with decrypted existing file (using hostKey),
    encrypt with host public key, output to `./secrets/renced/$host`
    and add to nix store.
    */
    pub fn renc(self, _all: bool, flake_root: PathBuf) -> Result<()> {
        let mut key_pair_list = self.get_key_pair_iter();
        info!(
            "rencrypt for host {}",
            self.settings.host_identifier.clone()
        );

        // check if flake root
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

        // absolute path, in config directory, suffix host ident
        let renc_path = {
            let mut p = flake_root.clone();
            p.push(self.settings.storage_dir_relative.clone());
            let p = p.canonicalize()?;
            info!(
                "reading user identity encrypted dir under flake root: {}",
                p.display()
            );
            p
        };

        // from secrets metadata, from real config store
        let data = Renc::new(
            self.secrets.clone(),
            renc_path.clone(),
            self.settings.host_pubkey.clone(),
        )
        .filter_exist();

        let parsed_ident = key_pair_list
            .find(|k| k.is_ok())
            .wrap_err_with(|| eyre!("available keypair not found"))??;

        let key = parsed_ident.get_identity();

        let recip = self.get_host_recip()?;
        if let Err(e) = data.map.makeup(vec![recip], &**key) {
            return Err(eyre!("makeup error: {}", e));
        } else {
            let o = add_to_store(renc_path)?;
            if !o.status.success() {
                error!("Command executed with failing error code");
            }
            // Another side, calculate with nix `builtins.path` and pass to when deploy as `storage`
            info!("path added to store: {}", String::from_utf8(o.stdout)?);
        }

        Ok(())
    }
}
