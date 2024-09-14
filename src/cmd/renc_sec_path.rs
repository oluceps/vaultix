use std::path::PathBuf;

use sha2::{Digest, Sha256};
use spdlog::{debug, info};

use crate::profile::{self, Settings};

pub struct RencSecretPath(PathBuf);

impl RencSecretPath {
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
        debug!("public key hash: {}", pubkey_hash);

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

            debug!("identity hash: {}", ident_hash);

            let mut storage_dir_path = PathBuf::from(storage_dir_store);
            info!("storage dir path prefix: {:?}", storage_dir_path);
            storage_dir_path.push(format!("{}-{}.age", ident_hash, name));
            storage_dir_path
        };
        Self(secret_file_path)
    }

    pub fn get(self) -> PathBuf {
        self.0
    }
}
