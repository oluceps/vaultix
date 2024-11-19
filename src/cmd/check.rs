use eyre::Result;
use log::{debug, error};

use crate::{
    helper::stored::{InStore, SecMap, SecPath},
    profile::Profile,
};

impl Profile {
    pub fn check(self) -> Result<()> {
        SecMap::<SecPath<_, InStore>>::create(&self.secrets)
            .renced_stored(
                self.settings.cache_in_store.clone().into(),
                self.settings.host_pubkey.as_str(),
            )
            .inner()
            .into_values()
            .try_for_each(|p| {
                debug!("checking in-store path: {}", p.path.display());
                if !p.path.exists() {
                    error!("path not found: {}", p.path.display());
                    error!("Forget adding it to git?");
                    error!("Please run renc and add new production to git");
                    error!("See https://oluceps.github.io/vaultix/nix-apps.html#renc");
                    return Err(eyre::eyre!("some secrets haven't been re-encrypted",));
                }
                Ok(())
            })
    }
}
