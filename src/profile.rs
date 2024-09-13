use eyre::Result;
use std::collections::HashMap;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub secrets: HashMap<String, Secret>,
    pub settings: Settings,
}

#[derive(Debug, Deserialize)]
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

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    pub decrypted_dir: String,
    pub decrypted_mount_point: String,
    pub extraEncryption_pubkeys: Vec<String>,
    pub host_pubkey: String,
    pub storage_dir: String,
    pub master_identities: Vec<MasterIdentity>,
}

#[derive(Debug, Deserialize)]
pub struct MasterIdentity {
    pub identity: String,
    pub pubkey: String,
}
