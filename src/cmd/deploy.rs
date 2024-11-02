use std::{
    collections::HashMap,
    fs::{self, DirEntry, File, OpenOptions, Permissions, ReadDir},
    io::{ErrorKind, Read, Write},
    iter,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    cmd::stored_sec_path::SecretPathMap,
    helper,
    profile::{self, HostKey, Profile},
};

use age::x25519;
use eyre::{eyre, Context, Result};
use spdlog::{debug, error, info, trace, warn};
use sys_mount::{Mount, MountFlags, SupportedFilesystems};

impl HostKey {
    pub fn get_identity(&self) -> Result<age::ssh::Identity> {
        fs::read_to_string(&self.path)
            .wrap_err_with(|| eyre!("reading ssh host key error: {}", self.path))
            .and_then(|i| {
                age::ssh::Identity::from_buffer(i.as_bytes(), None)
                    .map_err(|e| eyre!("convert age identity from ssh key error: {}", e))
            })
    }
}

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
            k.get_identity()
        } else {
            Err(eyre!("key with type {} not found", KEY_TYPE))
        }
    }
    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let res = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                // TODO: noswap mount tmpfs
                let support_ramfs =
                    SupportedFilesystems::new().and_then(|fss| Ok(fss.is_supported("ramfs")));
                if !support_ramfs? {
                    let err =
                        "ramfs not supported! Refusing extract secret since it will write to disk";
                    error!("{}", err);
                    return Err(eyre!(err));
                }
                let path = self.get_decrypted_mount_point_path();
                info!("creating mount point {}", path.clone());
                fs::create_dir_all(path.clone()).wrap_err_with(|| {
                    format!(
                        "creating decrypted mountpoint: {:?}",
                        self.get_decrypted_mount_point_path()
                    )
                })?;
                Mount::builder()
                    .fstype("ramfs")
                    .flags(MountFlags::NOSUID)
                    .data("relatime")
                    .data("mode=751")
                    .mount(String::default(), self.get_decrypted_mount_point_path())
                    .map(|_| ()) // not needed.
                    .wrap_err(eyre!("mount tmpfs error"))
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
                            debug!("found mountpoint generation {}", res);
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
            let map = SecretPathMap::from_profile(&self).inner();
            let mut ret = HashMap::new();
            map.into_iter().for_each(|(s, p)| {
                let _ = ret.insert(
                    s,
                    p.read_hostpubkey_encrypted_cipher_content().expect("error"),
                );
            });
            ret
        };

        trace!("{:?}", sec_ciphertext_map);

        let generation_count = self.init_decrypted_mount_point()?;

        let target_extract_dir_with_gen = {
            let mut p = PathBuf::from(self.get_decrypted_mount_point_path());
            p.push(generation_count.to_string());

            debug!("target extract dir with generation number: {:?}", p);

            fs::create_dir_all(&p)
                .map(|_| p)
                .wrap_err(eyre!(
                    "cannot create target extract dir with generation number"
                ))
                .and_then(|p| {
                    let _ = fs::set_permissions(&p, Permissions::from_mode(0o751))
                        .wrap_err(eyre!("set permission"));
                    Ok(p)
                })?
        };

        let decrypt_host_ident = &self.get_host_key_identity()?;

        sec_ciphertext_map.into_iter().for_each(|(n, c)| {
            let decrypted = {
                let decryptor = age::Decryptor::new(&c[..]).expect("parse cipher text error");

                let mut decrypted = vec![];
                let mut reader = decryptor
                    .decrypt(iter::once(decrypt_host_ident as &dyn age::Identity))
                    .expect("some");
                if let Err(e) = reader.read_to_end(&mut decrypted) {
                    error!("{}", e)
                };

                decrypted
            };

            info!("{} -> generation {}", n.name, generation_count);
            let mut the_file = {
                let mut p = target_extract_dir_with_gen.clone();
                p.push(n.name);

                let mode = helper::parse_permission::parse_octal_string(&n.mode).unwrap();
                let permissions = Permissions::from_mode(mode);

                let file = OpenOptions::new().create(true).write(true).open(p).unwrap();

                file.set_permissions(permissions).unwrap();

                helper::set_owner_group::set_owner_and_group(&file, &n.owner, &n.group)
                    .expect("good report");

                file
            };
            the_file
                .write_all(&decrypted)
                .expect("write decrypted file error")
        });

        let _ = fs::remove_file(self.get_decrypt_dir_path());
        // link back to /run/vaultix
        if std::os::unix::fs::symlink(target_extract_dir_with_gen, self.get_decrypt_dir_path())
            .wrap_err("create symlink error")
            .is_ok()
        {
            info!("deployment success");
        }
        Ok(())
    }
}
