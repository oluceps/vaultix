use std::{collections::HashMap, fs, path::PathBuf};

use eyre::Context;
use sha2::{Digest, Sha256};
use spdlog::{debug, info, trace};

use crate::profile::{self, Profile, Settings};

#[derive(Debug, Clone)]
pub struct StoredSecretPath(PathBuf);

#[derive(Debug, Clone)]
pub struct SecretPathMap(HashMap<profile::Secret, StoredSecretPath>);

pub struct SecretBufferMap(HashMap<profile::Secret, Vec<u8>>);

impl From<SecretPathMap> for SecretBufferMap {
    fn from(m: SecretPathMap) -> Self {
        let mut map = HashMap::new();
        m.inner().into_iter().for_each(|(s, p)| {
            let v = p.read_hostpubkey_encrypted_cipher_content().unwrap();
            map.insert(s, v);
        });
        Self(map)
    }
}
impl SecretBufferMap {
    pub fn inner(self) -> HashMap<profile::Secret, Vec<u8>> {
        self.0
    }
}

impl SecretPathMap {
    pub fn init_from_to_user_ident_encrypted_instore(profile: &Profile) -> Self {
        let mut m = HashMap::new();
        profile.secrets.iter().for_each(|(_, sec)| {
            m.insert(
                sec.clone(),
                StoredSecretPath(PathBuf::from(sec.file.clone())),
            );
        });
        Self(m)
    }
    pub fn init_from_to_renced_instore_path(profile: &Profile) -> Self {
        let mut m = HashMap::new();
        profile.secrets.clone().into_values().for_each(|s| {
            m.insert(s.clone(), s.to_renced_store_pathbuf(&profile.settings));
        });
        Self(m)
    }
    // v: flakeroot/secrets/renced/tester/hash-name.age
    pub fn to_flake_repo_relative_renced_path(
        &self,
        profile: &Profile,
        flake_root: PathBuf,
    ) -> Self {
        let renc_path = {
            let mut p = flake_root;
            p.push(profile.settings.storage_dir_relative.clone());
            p
        };
        let mut m = HashMap::new();
        profile.secrets.clone().into_values().for_each(|s| {
            let mut renc_path = renc_path.clone();
            let name = self
                .clone()
                .inner()
                .get(&s)
                .unwrap()
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            renc_path.push(name);
            // renc_path.canonicalize().unwrap();
            m.insert(s, StoredSecretPath(renc_path));
        });
        SecretPathMap(m)
    }
    pub fn inner(self) -> HashMap<profile::Secret, StoredSecretPath> {
        self.0
    }
    // pub fn get_all_secret_name(self) -> impl Iterator<Item = String> {
    //     self.inner().into_keys().map(|i| i.name.clone())
    // }
}

impl StoredSecretPath {
    pub fn init_from(settings: &Settings, secret: &profile::Secret) -> Self {
        let mut hasher = Sha256::new();
        let Settings {
            host_pubkey,
            storage_dir_store,
            ..
        } = settings;

        let pubkey_hash = {
            hasher.update(host_pubkey);
            format!("{:x}", hasher.clone().finalize())
        };
        trace!("public key hash: {}", pubkey_hash);

        let profile::Secret { file, name, .. } = secret;
        // TODO: here the storage_dir_path jiziwa no use
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

            trace!("identity hash: {}", ident_hash);

            let mut storage_dir_path = PathBuf::from(storage_dir_store);
            trace!("storage dir path prefix: {:?}", storage_dir_path);
            storage_dir_path.push(format!("{}-{}.age", ident_hash, name));
            trace!("added renced credential: {:?}", storage_dir_path);
            storage_dir_path
        };
        Self(secret_file_path)
    }

    pub fn read_hostpubkey_encrypted_cipher_content(self) -> eyre::Result<Vec<u8>> {
        trace!("reading cipher file: {:?}", self.0);
        fs::read(self.0).wrap_err(format!("read cipher file error"))
    }

    pub fn inner(self) -> PathBuf {
        self.0
    }
}
