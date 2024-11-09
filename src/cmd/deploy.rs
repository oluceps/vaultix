use std::{
    collections::HashMap,
    fs::{self, OpenOptions, Permissions, ReadDir},
    io::{ErrorKind, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    rc::Rc,
    str::FromStr,
};

use crate::{
    helper::{
        self,
        secret_buf::{Plain, SecBuf},
        stored::{InStore, SecMap, SecPath},
    },
    profile::{self, HostKey, Profile},
};

use age::{ssh, x25519, Recipient};
use eyre::{eyre, Context, Result};
use spdlog::{debug, error, info, trace};
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

fn deploy_to_fs(
    ctx: SecBuf<Plain>,
    item: impl crate::profile::DeployFactor,
    generation_count: usize,
    target_dir_ordered: PathBuf,
) -> Result<()> {
    info!("{} -> generation {}", item.get_name(), generation_count);
    let mut the_file = {
        let mut p = target_dir_ordered.clone();
        p.push(item.get_name().clone());

        let mode = helper::parse_permission::parse_octal_string(item.get_mode())
            .map_err(|e| eyre!("parse octal permission err: {}", e))?;
        let permissions = Permissions::from_mode(mode);

        let file = OpenOptions::new().create(true).write(true).open(p)?;

        file.set_permissions(permissions)?;

        helper::set_owner_group::set_owner_and_group(&file, item.get_owner(), item.get_group())?;

        file
    };
    the_file.write_all(ctx.buf_ref())?;
    Ok(())
}

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
            debug!("found host priv key: {:?}", k);
            k.get_identity()
        } else {
            Err(eyre!("key with type {} not found", KEY_TYPE))
        }
    }
    pub fn get_host_recip(&self) -> Result<Rc<dyn Recipient>> {
        let recip_str = self.settings.host_pubkey.as_str();
        macro_rules! try_recipients {
            ($pub_str:expr, $($type:path),+) => {
                $(
                    if let Ok(o) = <$type>::from_str($pub_str) {
                        return Ok(Rc::new(o) as Rc<dyn Recipient>);
                    }
                )+
            };
        }
        try_recipients!(recip_str, ssh::Recipient, x25519::Recipient);
        Err(eyre!("incompatible recipient type"))
    }
    pub fn _get_extra_recip(&self) -> Result<impl Iterator<Item = Box<dyn Recipient>>> {
        let extra_recips = self
            .settings
            .extra_recipients
            .iter()
            .map(|r| {
                age::x25519::Recipient::from_str(r.as_str())
                    .map(|r| Box::new(r) as Box<dyn Recipient>)
                    .map_err(|_| eyre!("parse extra recipient error"))
            })
            .collect::<Result<Vec<Box<dyn Recipient>>>>()?;
        Ok(extra_recips.into_iter())
    }

    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let res = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
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
        let host_prv_key = &self.get_host_key_identity()?;
        let plain_map: SecMap<Vec<u8>> = SecMap::<SecPath<_, InStore>>::create(&self.secrets)
            .renced_stored(
                self.settings.storage_in_store.clone().into(),
                self.settings.host_pubkey.as_str(),
            )
            .bake_ctx()?
            .inner()
            .into_iter()
            .map(|(s, c)| (s, c.decrypt(host_prv_key).expect("err").inner()))
            .collect();

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

        // deploy general secrets
        plain_map.inner_ref().iter().for_each(|(n, c)| {
            let ctx = SecBuf::<Plain>::new(c.clone());
            deploy_to_fs(
                ctx,
                *n,
                generation_count,
                target_extract_dir_with_gen.clone(),
            )
            .expect("err");
        });

        if self.templates.len() != 0 {
            info!("start deploy templates");
            use sha2::{Digest, Sha256};

            let get_hashed_id = |s: &profile::Secret| -> String {
                let mut hasher = Sha256::new();
                hasher.update(s.id.as_str());
                format!("{:X}", hasher.finalize()).to_lowercase()
            };

            // new map with sha256 hashed secret id str as key, ctx as value
            let hashstr_ctx_map: HashMap<String, &Vec<u8>> = plain_map
                .inner_ref()
                .iter()
                .map(|(k, v)| (get_hashed_id(*k), v))
                .collect();

            self.templates.clone().iter().for_each(|(_, t)| {
                // TODO:
                // parse content -> [hash]

                let mut raw_template = t.content.clone();

                let hashstrs_of_it = t.parse_hash_str_list().expect("parse template");

                hashstr_ctx_map
                    .iter()
                    .filter(|(k, _)| hashstrs_of_it.contains(*k))
                    .for_each(|(k, v)| {
                        // render
                        trace!("template before process: {}", raw_template);
                        raw_template = raw_template.replace(
                            format!("{{{{ {} }}}}", k).as_str(),
                            String::from_utf8_lossy(v).to_string().as_str(),
                        );
                        trace!("processed template: {}", raw_template);
                    });

                deploy_to_fs(
                    SecBuf::<Plain>::new(raw_template.into_bytes()),
                    t,
                    generation_count,
                    target_extract_dir_with_gen.clone(),
                )
                .expect("extract template to target generation")
            });
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ssh_host_pub_key() {
        // all 0x01
        let cipher_str = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
        if let Ok(_) = age::ssh::Recipient::from_str(&cipher_str) {
            assert!(true)
        } else {
            let _ = age::x25519::Recipient::from_str(&cipher_str).unwrap();
        }
    }
}
