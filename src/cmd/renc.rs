use age::{encrypted, x25519};
use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, info, trace};
use std::{
    collections::HashSet,
    fs::{self, File},
    io::{Read, Write},
    iter,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use crate::profile;
use crate::profile::{MasterIdentity, Profile, Settings};
use sha2::{digest::Key, Digest, Sha256};

struct RencSecretPath(PathBuf);

impl RencSecretPath {
    pub fn init_from(settings: &Settings, secret: &profile::Secret) -> Self {
        let mut hasher = Sha256::new();
        let Settings {
            host_pubkey,
            storage_dir,
            ..
        } = settings;

        let pubkey_hash = {
            hasher.update(host_pubkey);
            format!("{:x}", hasher.clone().finalize())
        };
        debug!("public key hash: {}", pubkey_hash);

        let profile::Secret { file, name, .. } = secret;
        let secret_file_path = {
            hasher.update(file);
            let secret_file_string_hash = format!("{:x}", hasher.clone().finalize());
            let ident_hash = {
                let mut pubkey_hash_string = pubkey_hash.clone();
                pubkey_hash_string.push_str(&secret_file_string_hash);
                let sum_hash_string = pubkey_hash_string;
                hasher.update(sum_hash_string);
                format!("{:x}", hasher.finalize()).split_off(32)
            };

            debug!("identity hash: {}", ident_hash);

            let mut storage_dir_path = PathBuf::from(storage_dir);
            storage_dir_path.push(format!("{}-{}.age", ident_hash, name));
            storage_dir_path
        };
        Self(secret_file_path)
    }
    pub fn get(self) -> PathBuf {
        self.0
    }
}

impl profile::Secret {
    fn to_renced_pathbuf(self, settings: &Settings) -> RencSecretPath {
        RencSecretPath::init_from(settings, &self)
    }
}

impl Profile {
    /// Get the `secrets.{}.file`, which in nix store
    pub fn get_cipher_file_paths(&self) -> HashSet<(String, PathBuf)> {
        let mut sec_set = HashSet::new();
        for (name, i) in &self.secrets {
            if sec_set.insert((name.to_owned(), PathBuf::from(i.file.clone()))) {
                debug!("found cipher file path {}", i.file)
            }
        }
        sec_set
    }

    /// Read
    pub fn get_cipher_contents(&self) -> HashSet<(String, Vec<u8>)> {
        self.get_cipher_file_paths()
            .iter()
            .map(|i| (i.to_owned().0, fs::read(i.to_owned().1).expect("yes")))
            .collect()
    }

    pub fn get_key_pair_list(
        &self,
    ) -> Vec<(Option<x25519::Identity>, eyre::Result<x25519::Recipient>)> {
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
            .collect()
    }

    /**
    First decrypt `./secrets/every` with masterIdentity's privkey,
    Then encrypt with host public key separately, output to
    `./secrets/renced/$host` and add to nix store.
    */
    pub fn renc(self, _all: bool) -> Result<()> {
        use age::ssh;
        let cipher_contents = self.get_cipher_contents();
        let renced_secret_paths: Vec<PathBuf> = self
            .secrets
            .clone()
            .into_values()
            .map(|i| i.to_renced_pathbuf(&self.settings).get())
            .collect();
        debug!("secret paths: {:?}", renced_secret_paths);
        // TODO: IMPL, renc need more element. host, masterIdent, pubhostkey, extraEncPubkey

        let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

        let key_pair_list = self.get_key_pair_list();
        // let encrypted = {
        //     let encryptor = age::Encryptor::with_recipients(vec![Box::new(
        //         key_pair_list
        //             .get(0)
        //             .clone()
        //             .unwrap()
        //             .1
        //             .as_ref()
        //             .cloned()
        //             .unwrap(),
        //     )])
        //     .expect("we provided a recipient");

        //     let mut encrypted = vec![];
        //     let mut writer = encryptor.wrap_output(&mut encrypted)?;
        //     writer.write_all(b"sometest")?;
        //     writer.finish()?;

        //     encrypted
        // };

        // debug!("{:?}", encrypted);

        if let Some(o) = key_pair_list.iter().find(|k| k.0.is_some()) {
            let key = o.0.clone().expect("some");
            let decrypted_file_ctnt = cipher_contents
                .iter()
                .map(|i| i.clone())
                .map(|i| {
                    let decryptor =
                        match age::Decryptor::new(&i.1[..]).expect("parse cipher text error") {
                            age::Decryptor::Recipients(d) => d,
                            _ => unreachable!(),
                        };

                    let mut decrypted = vec![];
                    let mut reader = decryptor
                        .decrypt(iter::once(&key as &dyn age::Identity))
                        .unwrap();

                    let _ = reader.read_to_end(&mut decrypted);
                    (i.0, decrypted)
                })
                .collect::<Vec<(String, Vec<u8>)>>();

            debug!("decrypted_file_ctnt: {:?}", decrypted_file_ctnt);
        };

        debug!("ssh recipients, host pubkey: {:?}", recip_host_pubkey);

        Ok(())
    }
}
