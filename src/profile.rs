use std::collections::HashMap;

use serde::Deserialize;

pub type SecretSet = HashMap<String, Secret>;
pub type TemplateSet = HashMap<String, Template>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub settings: Settings,
    pub secrets: SecretSet,
    pub templates: TemplateSet,
    pub need_by_user: Vec<String>,
    pub placeholder: PlaceHolderSet,
}

#[derive(Debug, Deserialize)]
pub struct PlaceHolderSet(pub HashMap<String, String>);

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Secret {
    pub id: String,
    pub file: String,
    pub group: String,
    pub mode: String,
    pub name: String,
    pub owner: String,
    pub path: String,
}

#[derive(Debug, Deserialize, Clone, Hash, Eq, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub name: String,
    pub content: String,
    pub trim: bool,
    pub group: String,
    pub mode: String,
    pub owner: String,
    pub path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub decrypted_dir: String,
    pub decrypted_dir_for_user: String,
    pub decrypted_mount_point: String,
    pub host_identifier: String,
    pub host_pubkey: String,
    pub host_keys: Vec<HostKey>,
    pub cache_in_store: String,
}

#[derive(Debug, Deserialize)]
pub struct HostKey {
    pub path: String,
    pub r#type: String,
}

pub trait DeployFactor {
    fn mode(&self) -> &String;
    fn owner(&self) -> &String;
    fn name(&self) -> &String;
    fn group(&self) -> &String;
    fn path(&self) -> &String;
}

macro_rules! impl_deploy_factor {
    ($type:ty, [ $($field:ident),+ $(,)? ]) => {
        impl DeployFactor for $type {
            $(
                fn $field(&self) -> &String {
                    &self.$field
                }
            )+
        }
    };
}

impl_deploy_factor!(&Secret, [mode, owner, name, group, path]);

impl_deploy_factor!(&Template, [mode, owner, name, group, path]);
