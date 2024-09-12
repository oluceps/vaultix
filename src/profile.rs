use std::collections::HashMap;

use serde::Deserialize;

pub type SecretSet = HashMap<String, Secret>;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub secrets: SecretSet,
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
pub struct Secret {
    pub id: String,
    pub file: String,
    pub group: String,
    pub mode: String,
    pub name: String,
    pub owner: String,
    pub path: String,
    pub symlink: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub decrypted_dir: String,
    pub decrypted_mount_point: String,
    pub host_identifier: String,
    #[allow(dead_code)]
    pub extra_recipients: Vec<String>,
    pub host_pubkey: String,
    pub host_keys: Vec<HostKey>,
    pub storage_dir_relative: String,
    pub storage_dir_store: String,
    pub master_identities: Vec<MasterIdentity>,
}

#[derive(Debug, Deserialize)]
pub struct MasterIdentity {
    pub identity: String,
    #[allow(dead_code)]
    pub pubkey: String,
}
#[derive(Debug, Deserialize)]
pub struct HostKey {
    pub path: String,
    pub r#type: String,
}
