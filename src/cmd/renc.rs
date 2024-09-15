use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info};
use std::{
    fs::{self, File},
    io::{Read, Write},
    iter,
    path::PathBuf,
    str::FromStr,
};

use crate::{
    cmd::stored_sec_path::{SecretBufferMap, SecretPathMap},
    profile::{MasterIdentity, Profile, Settings},
};
use crate::{interop::add_to_store, profile};

impl profile::Secret {
    pub fn to_renced_store_pathbuf(self, settings: &Settings) -> StoredSecretPath {
        StoredSecretPath::init_from(settings, &self)
    }
}

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct NamePathPair(String, PathBuf);

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct NamePathPairList(Vec<NamePathPair>);

use age::x25519;

use super::stored_sec_path::StoredSecretPath;
impl Profile {
    pub fn get_key_pair_iter<'a>(
        &'a self,
    ) -> impl Iterator<Item = (Option<x25519::Identity>, Result<x25519::Recipient>)> + 'a {
        use age::x25519;

        self.settings
            .master_identities
            .iter()
            .map(|MasterIdentity { identity, pubkey }| {
                if identity.is_empty() {
                    (
                        None,
                        x25519::Recipient::from_str(&pubkey)
                            .map_err(|i| eyre!("master identity pubkey {}", i)),
                    )
                } else {
                    let ident = fs::read_to_string(&identity)
                        .map_err(|_| eyre!("reading identity text error"))
                        .and_then(|i| {
                            // Omit comment
                            i.lines()
                                .last()
                                .map(|i| i.to_owned())
                                .wrap_err(eyre!("some"))
                        })
                        .and_then(|i| {
                            x25519::Identity::from_str(i.as_str())
                                .map_err(|_| eyre!("generating identity error"))
                        });

                    let recip = ident
                        .as_ref()
                        .map(x25519::Identity::to_public)
                        .map_err(|i| eyre!("convert identity to pubkey, {}", i));

                    (ident.ok(), recip)
                }
            })
    }

    /**
    First decrypt `./secrets/every` with masterIdentity's privkey,
    Then compare hash with decrypted existing file (using hostKey), encrypt with host public key, output to
    `./secrets/renced/$host` and add to nix store.
    */
    pub fn renc(self, _all: bool, flake_root: PathBuf) -> Result<()> {
        use age::ssh;

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

        let renc_path = {
            let mut p = flake_root;
            p.push(self.settings.storage_dir_relative.clone());
            info!(
                "reading user identity encrypted dir under flake root: {:?}",
                p
            );
            p
        };
        let mut key_pair_list = self.get_key_pair_iter();
        let sec_buf: SecretBufferMap =
            SecretPathMap::init_from_to_user_ident_encrypted_instore_file(&self).into();

        if let Some(o) = key_pair_list.find(|k| k.0.is_some()) {
            let key = o.0.clone().expect("some");
            let sec_buf = sec_buf.inner();
            let decrypted_iter = sec_buf.iter().map(|(s, b)| {
                let decryptor = match age::Decryptor::new(&b[..]).expect("parse cipher text error")
                {
                    age::Decryptor::Recipients(d) => d,
                    _ => unreachable!(),
                };

                let mut decrypted = vec![];
                let mut reader = decryptor
                    .decrypt(iter::once(&key as &dyn age::Identity))
                    .unwrap();

                let _ = reader.read_to_end(&mut decrypted);
                (s, decrypted)
            });

            let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

            let recip_unwrap = recip_host_pubkey.unwrap();

            let encrypted_iter = decrypted_iter.map(|(s, b)| {
                let encryptor =
                    age::Encryptor::with_recipients(vec![Box::new(recip_unwrap.clone())])
                        .expect("a recipient");
                let mut out_buf = vec![];

                let mut writer = encryptor.wrap_output(&mut out_buf).unwrap();

                writer.write_all(&b[..]).unwrap();
                writer.finish().unwrap();

                (s, out_buf)
            });

            debug!("re encrypted: {:?}", encrypted_iter);

            info!("cleaning old re-encryption extract dir");
            let _ = fs::remove_dir_all(&renc_path);
            fs::create_dir_all(&renc_path)?;
            let ren = SecretPathMap::init_from_to_renced_store_path(&self).inner();
            encrypted_iter.for_each(|(s, b)| {
                // let base_path = sec_path.clone().inner().get(s).cloned();

                let mut to_create = renc_path.clone();

                // if let Some(n) = base_path {
                // get store path from to_renced
                let renced_store_path = ren.get(s).cloned().unwrap().inner();
                to_create.push(renced_store_path.file_name().unwrap());

                debug!("path string {:?}", to_create);
                let mut fd = File::create(to_create).expect("create file error");
                let _ = fd.write_all(&b[..]);
                // }
            });

            let o = add_to_store(renc_path)?;
            if !o.status.success() {
                error!("Command executed with failing error code");
            }
            // Another side, calculate with nix `builtins.path` and pass to when deploy as `storage`
            info!("path added to store: {}", String::from_utf8(o.stdout)?);
        };

        Ok(())
    }
}
// Seems quite long huh
