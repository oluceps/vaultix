use std::path::PathBuf;

use eyre::{Context, ContextCompat, Result, eyre};
use log::debug;

use crate::util::secmap::{RencBuilder, RencCtx};

use super::renc::CompleteProfile;

impl CompleteProfile<'_> {
    pub fn check(&self, flake_root: PathBuf) -> Result<()> {
        let profile = self
            .inner_ref()
            .first()
            .with_context(|| eyre::eyre!("deploy must only one host"))?;

        let ctx = RencCtx::create(self, Some(flake_root))?;

        let inst = RencBuilder::create(self)
            .build_instore()
            .renced_stored(&ctx, profile.settings.cache_in_store.clone().into())
            .inner();

        inst.values().try_for_each(|p| {
            debug!("checking in-store path: {}", p.path.display());
            if !p.path.exists() {
                return Err(eyre!(
                    "See https://milieuim.github.io/vaultix/nix-apps.html#renc"
                ))
                .wrap_err_with(|| eyre!("Please run renc and add new production to git"))
                .wrap_err_with(|| eyre!("Forget adding it to git?"))
                .wrap_err_with(|| {
                    eyre::eyre!("secrets haven't been re-encrypted: {}", p.path.display())
                });
            }
            Ok(())
        })
    }
}
