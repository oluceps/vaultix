use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    iter,
    path::PathBuf,
    str::FromStr,
};

use crate::profile::{MasterIdentity, Profile, Settings};
use crate::{interop::add_to_store, profile};

impl profile::Secret {
    fn to_renced_pathbuf(self, settings: &Settings) -> StoredSecretPath {
        StoredSecretPath::init_from(settings, &self)
    }
}

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct NamePathPair(String, PathBuf);

#[derive(Hash, Debug, Eq, PartialEq)]
pub struct NamePathPairList(Vec<NamePathPair>);

impl NamePathPairList {
    pub fn inner(self) -> Vec<NamePathPair> {
        self.0
    }
    /// Vec<NamePathPair> => Map
    pub fn into_map(self) -> HashMap<String, PathBuf> {
        let mut renc_path_map = HashMap::new();
        for i in self.inner() {
            let _ = renc_path_map.insert(i.name(), i.path());
        }
        renc_path_map
    }
}

impl NamePathPair {
    fn name(&self) -> String {
        self.0.clone()
    }
    fn path(self) -> PathBuf {
        self.1
    }
}

#[derive(Hash, Debug, Eq, PartialEq, Clone)]
pub struct NameBufPair(String, Vec<u8>);

impl NameBufPair {
    fn name(&self) -> String {
        self.0.clone()
    }
    fn buf(self) -> Vec<u8> {
        self.1
    }
    fn from(raw: (String, Vec<u8>)) -> Self {
        Self(raw.0, raw.1)
    }
}

use age::x25519;

use super::stored_sec_path::StoredSecretPath;
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

    pub fn get_renced_paths(&self) -> NamePathPairList {
        NamePathPairList(
            self.secrets
                .clone()
                .into_values()
                .map(|i| NamePathPair(i.to_owned().id, i.to_renced_pathbuf(&self.settings).get()))
                .collect(),
        )
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
        let renced_secret_paths: NamePathPairList = self.get_renced_paths();
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
                raw.into_iter().map(|i| NameBufPair::from(i))
            };

            let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

            let recip_unwrap = recip_host_pubkey.unwrap();

            let encrypted = decrypted.map(|i| {
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

            let renc_path_map = renced_secret_paths.into_map();

            let renc_path = {
                let mut p = flake_root;
                p.push(self.settings.storage_dir_relative.clone());
                info!("reading dir {:?}", p);
                p
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
                    let _ = fd.write_all(&i.buf()[..]);
                }
            }
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
