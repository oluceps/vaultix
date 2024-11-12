use std::collections::HashMap;

use serde::Deserialize;

pub type SecretSet = HashMap<String, Secret>;
pub type TemplateSet = HashMap<String, Template>;

#[derive(Debug, Deserialize)]
pub struct Profile {
    pub secrets: SecretSet,
    pub settings: Settings,
    pub templates: TemplateSet,
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

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
pub struct Template {
    pub name: String,
    pub content: String,
    pub trim: bool,
    pub group: String,
    pub mode: String,
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
    pub host_pubkey: String,
    pub host_keys: Vec<HostKey>,
    pub storage_location: String,
    pub storage_in_store: String,
}

#[derive(Debug, Deserialize)]
pub struct HostKey {
    pub path: String,
    pub r#type: String,
}

pub trait DeployFactor {
    fn get_mode(&self) -> &String;
    fn get_owner(&self) -> &String;
    fn get_name(&self) -> &String;
    fn get_group(&self) -> &String;
}

macro_rules! impl_deploy_factor {
    ($type:ty, { $($method:ident => $field:ident),+ $(,)? }) => {
        impl DeployFactor for $type {
            $(
                fn $method(&self) -> &String {
                    &self.$field
                }
            )+
        }
    };
}

impl_deploy_factor!(&Secret, {
    get_mode => mode,
    get_owner => owner,
    get_name => name,
    get_group => group
});
impl_deploy_factor!(&Template, {
    get_mode => mode,
    get_owner => owner,
    get_name => name,
    get_group => group
});
