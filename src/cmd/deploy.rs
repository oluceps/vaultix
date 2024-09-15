use std::{
    collections::HashMap,
    fs::{self, DirEntry, File, ReadDir},
    io::{ErrorKind, Read, Write},
    iter,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    cmd::stored_sec_path::SecretPathMap,
    profile::{self, Profile},
};

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
                            if res >= max {
                                max = res + 1;
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
        // secrets => vec<u8>
        let sec_ciphertext_map: HashMap<profile::Secret, Vec<u8>> = {
            let map = SecretPathMap::init_from(&self).inner();
            let mut ret = HashMap::new();
            map.into_iter().for_each(|(s, p)| {
                let _ = ret.insert(s, p.read_to_cipher_content().expect("read error"));
            });
            ret
        };

        trace!("{:?}", sec_ciphertext_map);

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

        sec_ciphertext_map.into_iter().for_each(|(n, c)| {
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
                p.push(n.name);
                File::create(p)
            }
            .expect("create file error");
            // TODO: permission and so on
            the_file_fd
                .write_all(&decrypted)
                .expect("write decrypted file error")
        });

        let _ = fs::remove_file(self.get_decrypt_dir_path());
        // link back to /run/vaultix
        if std::os::unix::fs::symlink(target_extract_dir_with_gen, self.get_decrypt_dir_path())
            .wrap_err("create symlink error")
            .is_ok()
        {
            info!("deploy secrets success");
        }
        Ok(())
    }
}
