use eyre::Result;
use spdlog::{debug, error};

use crate::{
    helper::stored::{InStore, SecMap, SecPath},
    profile::Profile,
};

impl Profile {
    pub fn check(self) -> Result<()> {
        SecMap::<SecPath<_, InStore>>::create(&self.secrets)
            .renced_stored(
                self.settings.storage_in_store.clone().into(),
                self.settings.host_pubkey.as_str(),
            )
            .inner()
            .into_values()
            .map(|p| {
                debug!("checking in-store path: {}", p.path.display());
                if !p.path.exists() {
                    error!("path not found: {}\nTry run renc app", p.path.display());
                    return Err(eyre::eyre!("rencypted secret not in expected location",));
                }
                Ok(())
            })
            .collect()
    }
}
