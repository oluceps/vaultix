use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info, trace};
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

use age::{x25519, Identity};

use super::stored_sec_path::StoredSecretPath;
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
            info!(
                "reading user identity encrypted dir under flake root: {}",
                p.display()
            );
            p
        };

        // from secrets metadata, read real config store
        let sec_buf: SecretBufferMap = SecretPathMap::from_secret_set(&self.secrets).into();

        let decrypt = |buffer: &Vec<u8>, key: &dyn Identity| -> Result<Vec<u8>> {
            let decryptor = age::Decryptor::new(&buffer[..])?;

            let mut decrypted = vec![];
            let mut reader = decryptor.decrypt(iter::once(key as &dyn age::Identity))?;
            let res = reader.read_to_end(&mut decrypted);
            if let Ok(b) = res {
                debug!("decrypted secret {} bytes", b);
            }

            Ok(decrypted)
        };

        // WARN: this failed while using plugin
        let avail_ident = if let Some(found) = key_pair_list.find(|k| k.is_ok()) {
            if let Ok(r) = found {
                r
            } else {
                return Err(eyre!("no available key for this material"));
            }
        } else {
            return Err(eyre!("provided identities not valid"));
        };

        let key = avail_ident.get_identity();
        let sec_buf = sec_buf.inner();
        let decrypted_iter = sec_buf.iter().filter_map(|(s, b)| {
            let decrypted = decrypt(b, &**key);
            Some((s, decrypted))
        });

        let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

        let recip_unwrap = recip_host_pubkey.unwrap();

        let encrypted_iter = decrypted_iter.filter_map(|(s, b)| {
            let m = SecretPathMap::from_profile(&self)
                .to_flake_repo_relative_renced_path(&self, flake_root.clone());
            let buffer = if let Err(e) = b {
                error!("{}", e);
                return None;
            } else {
                b.expect("there")
            };

            let b_hash = blake3::hash(&buffer);

            if let Some(o) = m.inner().get(s) {
                let flake_renc = fs::read(o.clone().inner());
                if let Ok(c) = flake_renc {
                    let ctx = {
                        let decryptor = age::Decryptor::new(&c[..]).expect("");

                        let mut decrypted = vec![];
                        let mut reader = decryptor
                            .decrypt(iter::once(
                                &self
                                    .settings
                                    .host_keys // TODO: cannot compare on remote machine
                                    .get(0)
                                    .unwrap()
                                    .get_identity()
                                    .unwrap() as &dyn age::Identity,
                            ))
                            .expect("decrypt fail");
                        let res = reader.read_to_end(&mut decrypted);
                        if let Ok(b) = res {
                            debug!("decrypted secret in store: {} bytes", b);
                        }

                        decrypted
                    };
                    trace!("checking hash{:?}", c);

                    let c_hash = blake3::hash(&ctx);
                    trace!("hash: prev {} after {}", c_hash, b_hash);
                    if c_hash == b_hash {
                        // skip
                        info!("skip unchanged file: {}", s.name);
                        return None;
                    }
                }
            }

            let encryptor = age::Encryptor::with_recipients(iter::once(&recip_unwrap as _))
                .expect("a recipient");
            let mut out_buf = vec![];

            let mut writer = encryptor.wrap_output(&mut out_buf).unwrap();

            writer.write_all(&buffer[..]).unwrap();
            writer.finish().unwrap();

            Some((s, out_buf))
        });

        trace!("re encrypted: {:?}", encrypted_iter);

        info!("cleaning old re-encryption extract dir");
        let ren = SecretPathMap::from_profile(&self).inner();
        encrypted_iter.for_each(|(s, b)| {
            let mut to_create = renc_path.clone();

            let renced_store_path = ren.get(s).cloned().unwrap().inner();
            to_create.push(renced_store_path.file_name().unwrap());

            debug!("path string {:?}", to_create);
            let mut fd = File::create(to_create).expect("create file error");
            let _ = fd.write_all(&b[..]);
        });

        let o = add_to_store(renc_path)?;
        if !o.status.success() {
            error!("Command executed with failing error code");
        }
        // Another side, calculate with nix `builtins.path` and pass to when deploy as `storage`
        info!("path added to store: {}", String::from_utf8(o.stdout)?);
        Ok(())
    }
}
