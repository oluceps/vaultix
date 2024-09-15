use std::{
    collections::HashMap,
    fs::{self, DirEntry, ReadDir},
    io::ErrorKind,
    path::{Path, PathBuf},
};

use crate::profile::Profile;

use eyre::{eyre, Context, Result};
use spdlog::{debug, error, info};

impl Profile {
    pub fn get_decrypted_mount_point_path(&self) -> String {
        self.settings.decrypted_mount_point.to_string()
    }
    pub fn get_decrypt_dir_path(&self) -> String {
        self.settings.decrypted_dir.to_string()
    }
    pub fn read_decrypted_mount_point(&self) -> std::io::Result<ReadDir> {
        fs::read_dir(self.get_decrypted_mount_point_path())
    }
    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let b = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                fs::create_dir_all(self.get_decrypted_mount_point_path())
                    .wrap_err("create decrypted mountpoint error")
            }
            Err(e) => {
                error!("{}", e);
                Err(e).wrap_err(eyre!("read mountpoint error"))
            }
            Ok(o) => {
                o.for_each(|en| {
                    match str::parse::<usize>(
                        en.unwrap()
                            .file_name()
                            .to_string_lossy()
                            .to_string()
                            .as_str(),
                    ) {
                        Err(e) => {
                            error!("parse mount point generation err: {:?}", e)
                        }
                        Ok(res) => {
                            info!("found mountpoint generation {}", res);
                            if res > max {
                                max = res;
                            }
                        }
                    }
                });
                Ok(())
            }
        };

        Ok(max)
    }
    /**
    extract secrets to `/run/vaultix.d/$num` and link to `/run/vaultix`
    */
    pub fn deploy(self) -> Result<()> {
        let storage_name_ctt_map: HashMap<String, Vec<u8>> = {
            let mut map = HashMap::new();
            // dir with host pub key encrypted material, prefix hash
            let storage = PathBuf::from(&self.settings.storage_dir_store);
            fs::read_dir(storage)?.for_each(|entry| {
                let entry = entry.expect("enter store, must success");
                let path = entry
                    .path()
                    .canonicalize()
                    .expect("file path initialize error");
                let name = entry.file_name().to_string_lossy().to_string();
                debug!("record secret name from store: {}", name);
                let content = fs::read(path).expect("reading store, must success");
                map.insert(name, content);
            });
            map
        };

        let secs_map = self.get_renced_store_paths().into_map();

        for s in secs_map.values().into_iter() {
            debug!("found cipher file {:?}", s.canonicalize()?);
        }

        Ok(())
    }
}
