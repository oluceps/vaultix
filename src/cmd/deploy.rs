use std::path::{Path, PathBuf};

use crate::profile::Profile;

use eyre::Result;

impl Profile {
    pub fn deploy<P>(self, flake_root: P, storage: P) -> Result<()>
    where
        P: AsRef<Path> + Into<PathBuf>,
    {
        Ok(())
    }
}
