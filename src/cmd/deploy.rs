use std::path::{Path, PathBuf};

use crate::profile::Profile;

use eyre::Result;
use spdlog::debug;

impl Profile {
    pub fn deploy<P>(self, _flake_root: P, storage: P) -> Result<()>
    where
        P: AsRef<Path> + Into<PathBuf>,
    {
        let storage = storage.as_ref().to_path_buf();

        let secs_map = self.get_renced_paths().into_map();

        for s in secs_map.values().into_iter() {
            debug!("found cipher file {:?}", s);
        }

        Ok(())
    }
}
