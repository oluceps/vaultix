use crate::{
    interop::add_to_store,
    parser::identity::{ParsedIdentity, RawIdentity},
    profile::Profile,
    util::secmap::{RencBuilder, RencCtx},
};
use age::Identity;
use eyre::{eyre, Result};
use log::{error, info};
use std::{fs, path::PathBuf};

pub struct CompleteProfile<'a>(pub Vec<&'a Profile>);

impl<'a> FromIterator<&'a Profile> for CompleteProfile<'a> {
    fn from_iter<T: IntoIterator<Item = &'a Profile>>(iter: T) -> Self {
        iter.into_iter().collect()
    }
}

impl<'a> CompleteProfile<'a> {
    pub fn _inner(self) -> Vec<&'a Profile> {
        self.0
    }
    pub fn inner_ref(&self) -> &Vec<&Profile> {
        &self.0
    }
    /**
    read secret metadata from profile

    First decrypt `./secrets/every` with masterIdentity's privkey,
    Then compare hash with decrypted existing file (using hostKey),
    encrypt with host public key, output to `./secrets/renced/$host`
    and add to nix store.
    */
    pub fn renc(self, flake_root: PathBuf, identity: String, cache_path: PathBuf) -> Result<()> {
        // check if flake root
        if !fs::read_dir(&flake_root)?.any(|e| {
            e.is_ok_and(|ie| {
                ie.file_name()
                    .into_string()
                    .is_ok_and(|iie| iie.as_str() == "flake.nix")
            })
        }) {
            error!("please run app in flake root");
            return Err(eyre!(
                "`flake.nix` not found here, make sure run in flake toplevel."
            ));
        };

        let ctx = RencCtx::create(&self);

        let instance = {
            let mut ret = RencBuilder::create(&self).build_inrepo(&ctx, cache_path.clone());
            ret.clean_outdated(cache_path.clone())?;

            ret.retain_noexist();
            ret
        };

        let key_pair: ParsedIdentity = RawIdentity::from(identity).try_into()?;

        info!("re-encrypting...");
        instance.makeup(&ctx, key_pair.identity.as_ref() as &dyn Identity)?;
        info!("finish.");

        instance
            .all_host_cache_in_repo(cache_path)
            .into_iter()
            .try_for_each(|i| {
                info!("adding cache to store: {}", i.display());
                let o = add_to_store(i)?;
                if !o.status.success() {
                    error!("Command executed with failing error code");
                    // Another side, calculate with nix `builtins.path` and pass to when deploy as `storage`
                    info!("path added to store: {}", String::from_utf8(o.stdout)?);
                    return Err(eyre!("unexpected error"));
                }
                Ok(())
            })
    }
}
