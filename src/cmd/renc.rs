use blake3::Hasher;
use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info, trace};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    iter,
    path::PathBuf,
    str::FromStr,
};

use crate::{
    cmd::stored_sec_path::{HashWithCtx, InCfg, SecMap, SecPath},
    profile::{MasterIdentity, Profile},
};
use crate::{interop::add_to_store, profile};

use age::{x25519, Identity};

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
        use age::ssh;
        let mut key_pair_list = self.get_key_pair_iter();

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
        let data = SecMap::<SecPath<_, InCfg>>::from(self.secrets.clone(), renc_path.clone());

        let data_renc_path_map = data.clone().calc_renc(self.settings.host_pubkey.clone())?;

        let parsed_ident = key_pair_list
            .find(|k| k.is_ok())
            .wrap_err_with(|| eyre!("available keypair not found"))??;

        let key = parsed_ident.get_identity();

        let decrypt = |buffer: &Vec<u8>| -> Result<Vec<u8>> {
            let decryptor = age::Decryptor::new(&buffer[..])?;

            let mut decrypted = vec![];
            let mut reader = decryptor.decrypt(iter::once(&**key))?;
            let res = reader.read_to_end(&mut decrypted);
            if let Ok(b) = res {
                debug!("decrypted secret {} bytes", b);
            }

            Ok(decrypted)
        };

        let sec_need_renc = data_renc_path_map
            .inner()
            .into_iter()
            .filter(|(k, v)| {
                // TODO: extraReceip
                let hash = v.get_hash();
                let renc_path = {
                    let mut path = renc_path.clone();
                    path.push(hash.to_string());
                    path.canonicalize().expect("no err")
                };

                debug!("comparing {}", renc_path.display());

                let exs = renc_path.exists();

                if exs {
                    info!("skipping {} since exist", k.id)
                }

                !exs
            })
            .collect::<HashMap<profile::Secret, HashWithCtx>>()
            .into_keys()
            .collect::<Vec<profile::Secret>>();

        // TODO: host pub key type safe
        data.makeup(sec_need_renc, self.settings.host_pubkey.clone(), decrypt)

        // let o = add_to_store(renc_path)?;
        // if !o.status.success() {
        //     error!("Command executed with failing error code");
        // }
        // // Another side, calculate with nix `builtins.path` and pass to when deploy as `storage`
        // info!("path added to store: {}", String::from_utf8(o.stdout)?);
        // Ok(())
    }
}
