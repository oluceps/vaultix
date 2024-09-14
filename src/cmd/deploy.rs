use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::profile::Profile;

use eyre::Result;
use spdlog::debug;

impl Profile {
    pub fn deploy(self) -> Result<()> {
        let storage_name_ctt_map: HashMap<String, Vec<u8>> = {
            let mut map = HashMap::new();
            // dir with host pub key encrypted material, prefix hash
            let storage = PathBuf::from(&self.settings.storage_dir_store);
            fs::read_dir(storage)?.for_each(|entry| {
                let entry = entry.expect("enter store, must success");
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();
                debug!("record secret name from store: {}", name);
                let content = fs::read(path).expect("reading store, must success");
                map.insert(name, content);
            });
            map
        };

        // for entry in storage_ctt {
        //     let entry = entry?;
        //     let path = entry.path();

        //     debug!("found renced secret in store: {:?}", path);
        // }

        let secs_map = self.get_renced_paths().into_map();

        for s in secs_map.values().into_iter() {
            debug!("found cipher file {:?}", s);
        }

        Ok(())
    }
}
