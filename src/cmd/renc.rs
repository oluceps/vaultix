use crate::{
    parser::identity::{ParsedIdentity, RawIdentity},
    profile::Profile,
    util::secmap::{RencBuilder, RencCtx},
};
use eyre::{Result, bail};
use log::error;
use std::{fs, path::PathBuf};

pub struct CompleteProfile<'a>(pub Vec<&'a Profile>);

impl<'a> FromIterator<&'a Profile> for CompleteProfile<'a> {
    fn from_iter<T: IntoIterator<Item = &'a Profile>>(iter: T) -> Self {
        let mut v = Vec::new();
        for i in iter.into_iter() {
            v.push(i);
        }
        Self(v)
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
            bail!("`flake.nix` not found here, make sure run in flake toplevel.");
        };

        let ctx = RencCtx::create(&self, None)?;
        let mut materia = RencBuilder::create(&self).build_inrepo(&ctx, cache_path.clone());
        materia.clean_outdated(cache_path)?;
        materia.retain_noexist();

        let ParsedIdentity {
            identity,
            recipient: _,
        } = RawIdentity::from(identity).try_into()?;

        materia.build_instance().makeup(&ctx, identity)
    }
}
