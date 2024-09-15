use std::{
    collections::HashMap,
    fs::{self, DirEntry, File, ReadDir},
    io::{ErrorKind, Read, Write},
    iter,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::profile::Profile;

use age::x25519;
use eyre::{eyre, Context, Result};
use spdlog::{debug, error, info, trace};

const KEY_TYPE: &str = "ed25519";
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

    pub fn get_host_key_identity(&self) -> Result<age::ssh::Identity> {
        if let Some(k) = self
            .settings
            .host_keys
            .iter()
            .find(|i| i.r#type == KEY_TYPE)
        {
            fs::read_to_string(&k.path)
                .wrap_err_with(|| eyre!("reading ssh host key error: {}", k.path))
                .and_then(|i| {
                    age::ssh::Identity::from_buffer(i.as_bytes(), Some(String::from("thekey")))
                        .map_err(|e| eyre!("convert age identity from ssh key error: {}", e))
                })
        } else {
            Err(eyre!("key with type {} not found", KEY_TYPE))
        }
    }
    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let res = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                fs::create_dir_all(self.get_decrypted_mount_point_path()).wrap_err_with(|| {
                    format!(
                        "creating decrypted mountpoint: {:?}",
                        self.get_decrypted_mount_point_path()
                    )
                })
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

        res.map(|_| max)
    }
    /**
    extract secrets to `/run/vaultix.d/$num` and link to `/run/vaultix`
    */
    pub fn deploy(self) -> Result<()> {
        // hash-name.age => vec<u8>
        let name_ciphertext_map: HashMap<String, Vec<u8>> = {
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

        trace!("{:?}", name_ciphertext_map);

        let generation_count = self.init_decrypted_mount_point()?;

        let target_extract_dir_with_gen = {
            let mut p = PathBuf::from(self.get_decrypted_mount_point_path());
            p.push(generation_count.to_string());

            debug!("target extract dir with generation number: {:?}", p);

            fs::create_dir_all(&p).map(|_| p).wrap_err(eyre!(
                "cannot create target extract dir with generation number"
            ))?
        };

        let decrypt_host_ident = &self.get_host_key_identity()?;

        name_ciphertext_map.into_iter().for_each(|(n, c)| {
            let decryptor = match age::Decryptor::new(&c[..]).expect("parse cipher text error") {
                age::Decryptor::Recipients(d) => d,
                _ => unreachable!(),
            };

            let mut decrypted = vec![];

            let mut reader = decryptor
                .decrypt(iter::once(decrypt_host_ident as &dyn age::Identity))
                .unwrap();

            let _ = reader.read_to_end(&mut decrypted);

            let mut the_file_fd = {
                let mut p = target_extract_dir_with_gen.clone();
                p.push(n);
                File::create(p)
            }
            .expect("create file error");
            the_file_fd
                .write_all(&decrypted)
                .expect("write decrypted file error")
        });

        // link back to /run/vaultix
        std::os::unix::fs::symlink(target_extract_dir_with_gen, self.get_decrypt_dir_path())
            .wrap_err("create symlink error")
    }
}
