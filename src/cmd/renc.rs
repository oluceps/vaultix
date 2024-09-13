use std::path::{Path, PathBuf};

use eyre::Result;
use spdlog::{debug, trace};

use crate::profile::{Profile, Secret, Settings};
use sha2::{Digest, Sha256};

struct RencSecretPath(PathBuf);

impl RencSecretPath {
    pub fn init_from(settings: &Settings, secret: &Secret) -> Self {
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
        debug!("{}", pubkey_hash);

        let Secret { file, name, .. } = secret;
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

            debug!("{}", ident_hash);

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

impl Secret {
    fn to_pathbuf(self, settings: &Settings) -> RencSecretPath {
        RencSecretPath::init_from(settings, &self)
    }
}

impl Profile {
    pub fn renc(self, all: bool) -> Result<()> {
        let secret_paths: Vec<PathBuf> = {
            let secret_list: Vec<PathBuf> = self
                .secrets
                .into_values()
                .map(|i| i.to_pathbuf(&self.settings).get())
                .collect();
            secret_list
        };
        Ok(())
    }
}
