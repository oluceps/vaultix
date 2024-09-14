use eyre::Result;
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub secrets: HashMap<String, Secret>,
    pub settings: Settings,
}

#[derive(Debug, Deserialize, Clone)]
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
    pub extraEncryption_pubkeys: Vec<String>,
    pub host_pubkey: String,
    pub host_keys: Vec<HostKey>,
    pub storage_dir_relative: String,
    pub storage_dir_store: String,
    pub master_identities: Vec<MasterIdentity>,
}

#[derive(Debug, Deserialize)]
pub struct MasterIdentity {
    pub identity: String,
    pub pubkey: String,
}
#[derive(Debug, Deserialize)]
pub struct HostKey {
    pub path: String,
    pub r#type: String,
}
