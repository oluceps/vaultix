use age::{encrypted, x25519};
use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, info, trace};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsStr,
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

const SECRET_DIR: &str = "secrets";

struct RencSecretPath(PathBuf);

impl RencSecretPath {
    pub fn init_from(settings: &Settings, secret: &profile::Secret) -> Self {
        let mut hasher = Sha256::new();
        let Settings {
            host_pubkey,
            storage_dir_suffix,
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

            let mut storage_dir_path = PathBuf::from(storage_dir_suffix);
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

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct NamePathPair(String, PathBuf);

impl NamePathPair {
    fn name(self) -> String {
        self.0
    }
    fn path(self) -> PathBuf {
        self.1
    }

    fn get_base_path(&self) -> Option<&OsStr> {
        self.1.file_name()
    }
}

#[derive(Hash, Debug, Eq, PartialEq, Clone)]
pub struct NameBufPair(String, Vec<u8>);

impl NameBufPair {
    fn name(&self) -> String {
        self.0.clone()
    }
    fn path(self) -> Vec<u8> {
        self.1
    }
    fn from(raw: (String, Vec<u8>)) -> Self {
        Self(raw.0, raw.1)
    }
}

impl Profile {
    /// Get the `secrets.{}.file`, which in nix store
    pub fn get_cipher_file_paths(&self) -> HashSet<NamePathPair> {
        let mut sec_set = HashSet::new();
        for (name, i) in &self.secrets {
            if sec_set.insert(NamePathPair(name.to_owned(), PathBuf::from(i.file.clone()))) {
                debug!("found cipher file path {}", i.file)
            }
        }
        sec_set
    }

    /// Read
    pub fn get_cipher_contents(&self) -> HashSet<NameBufPair> {
        self.get_cipher_file_paths()
            .iter()
            .map(|i| NameBufPair(i.0.clone(), fs::read(i.1.clone()).expect("yes")))
            .collect()
    }

    pub fn get_key_pair_list<'a>(
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
    Then encrypt with host public key separately, output to
    `./secrets/renced/$host` and add to nix store.
    */
    pub fn renc(self, _all: bool, flake_root: PathBuf) -> Result<()> {
        use age::ssh;
        let cipher_contents = self.get_cipher_contents();
        let renced_secret_paths: Vec<NamePathPair> = self
            .secrets
            .clone()
            .into_values()
            .map(|i| NamePathPair(i.to_owned().id, i.to_renced_pathbuf(&self.settings).get()))
            .collect();
        debug!("secret paths: {:?}", renced_secret_paths);

        let mut key_pair_list = self.get_key_pair_list();

        if let Some(o) = key_pair_list.find(|k| k.0.is_some()) {
            let key = o.0.clone().expect("some");
            let decrypted = {
                let raw = cipher_contents
                    .iter()
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
                        (i.name(), decrypted)
                    })
                    .collect::<Vec<(String, Vec<u8>)>>();
                raw.into_iter()
                    .map(|i| NameBufPair::from(i))
                    .collect::<Vec<NameBufPair>>()
            };
            debug!("decrypted_file_ctnt: {:?}", decrypted);

            let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

            let recip_unwrap = recip_host_pubkey.unwrap();

            let encrypted = decrypted.into_iter().map(|i| {
                let encryptor =
                    age::Encryptor::with_recipients(vec![Box::new(recip_unwrap.clone())])
                        .expect("a recipient");
                let NameBufPair(name, buf) = i;
                let mut out_buf = vec![];

                let mut writer = encryptor.wrap_output(&mut out_buf).unwrap();

                writer.write_all(&buf[..]).unwrap();
                writer.finish().unwrap();

                NameBufPair(name, out_buf)
            });
            debug!("re encrypted: {:?}", encrypted);

            let renc_path_map = {
                let mut renc_path_map = HashMap::new();
                for i in renced_secret_paths {
                    let _ = renc_path_map.insert(i.0, i.1);
                }
                renc_path_map
            };

            let renc_path = {
                let mut p = flake_root;
                p.push(self.settings.storage_dir_suffix.clone());
                p.canonicalize()?
            };
            if !renc_path.exists() {
                let _ = fs::create_dir_all(&renc_path);
            }
            for i in encrypted {
                let base_path = renc_path_map.get(i.name().as_str());

                let mut to_create = renc_path.clone();

                if let Some(n) = base_path {
                    to_create.push(n.file_name().unwrap());

                    debug!("path string {:?}", to_create);
                    let mut fd = File::create(to_create)?;
                    let _ = fd.write_all(&i.1[..]);
                }
            }
        };

        Ok(())
    }
}
