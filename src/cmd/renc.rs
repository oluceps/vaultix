use eyre::{eyre, Context, Result};
use spdlog::{debug, error, info};
use std::{fs, path::PathBuf};

use crate::helper::stored::Renc;
use crate::interop::add_to_store;
use crate::profile::Profile;

use crate::helper::parse_identity::ParsedIdentity;
impl Profile {
    pub fn get_parsed_ident(&self) -> Result<ParsedIdentity> {
        self.settings.identity.clone().try_into()
    }

    /**
    read secret metadata from profile

    First decrypt `./secrets/every` with masterIdentity's privkey,
    Then compare hash with decrypted existing file (using hostKey),
    encrypt with host public key, output to `./secrets/renced/$host`
    and add to nix store.
    */
    pub fn renc(self, flake_root: PathBuf) -> Result<()> {
        info!(
            "rencrypt for host [{}]",
            self.settings.host_identifier.clone()
        );

        // check if flake root
        if !fs::read_dir(&flake_root)?.any(|e| {
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
            p.push(self.settings.storage_location.clone());
            // pretend err is not found
            if p.canonicalize().is_err() {
                fs::create_dir_all(&p).wrap_err_with(|| eyre!("create storageLocation error"))?
            };
            p.canonicalize()?;
            debug!(
                "reading user identity encrypted dir under flake root: {}",
                p.display()
            );
            p
        };

        // from secrets metadata, from real config store
        let data = Renc::create(
            &self.secrets,
            renc_path.clone(),
            self.settings.host_pubkey.as_str(),
        )
        .filter_exist();

        let key_pair = self.get_parsed_ident()?;
        let key = key_pair.get_identity();

        let recip = self.get_host_recip()?;
        if let Err(e) = data.map.makeup(vec![recip], key) {
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
