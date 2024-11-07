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
                    error!("path not found: {}", p.path.display());
                    error!("Please run renc");
                    error!("See https://github.com/oluceps/vaultix/tree/main?tab=readme-ov-file#nix-app-renc");
                    return Err(eyre::eyre!("some secrets haven't been re-encrypted",));
                }
                Ok(())
            })
            .collect()
    }
}
