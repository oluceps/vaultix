use eyre::Result;
use spdlog::{debug, info, trace};
use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

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
        debug!("public key hash: {}", pubkey_hash);

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

impl Secret {
    fn to_pathbuf(self, settings: &Settings) -> RencSecretPath {
        RencSecretPath::init_from(settings, &self)
    }
}

impl Profile {
    pub fn renc(self, all: bool) -> Result<()> {
        use age::ssh;
        let secret_paths: Vec<PathBuf> = self
            .secrets
            .into_values()
            .map(|i| i.to_pathbuf(&self.settings).get())
            .collect();
        debug!("secret paths: {:?}", secret_paths);
        // TODO: IMPL, renc need more element. host, masterIdent, pubhostkey, extraEncPubkey

        let recip_host_pubkey = ssh::Recipient::from_str(self.settings.host_pubkey.as_str());

        debug!("age ssh recipients: {:?}", recip_host_pubkey);

        Ok(())
    }
}
