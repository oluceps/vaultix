use std::{
    collections::HashMap,
    fs::{self, OpenOptions, Permissions, ReadDir},
    io::{ErrorKind, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    rc::Rc,
};

use crate::{
    helper::{
        self,
        secret_buf::{Plain, SecBuf},
        stored::{InStore, SecMap, SecPath},
    },
    profile::{self, HostKey, Profile},
};

use crate::helper::parse_recipient::RawRecip;
use age::Recipient;
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

        let mode = crate::parser::parse_octal_str(item.get_mode())
            .map_err(|e| eyre!("parse octal permission err: {}", e))?;
        let permissions = Permissions::from_mode(mode);

        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(p)?;

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
        let recip: RawRecip = self.settings.host_pubkey.clone().into();
        recip.try_into()
    }

    /// init decrypted mount point and return the generation count
    pub fn init_decrypted_mount_point(&self) -> Result<usize> {
        let mut max = 0;
        let res = match self.read_decrypted_mount_point() {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let support_ramfs =
                    SupportedFilesystems::new().map(|fss| fss.is_supported("ramfs"));
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
            Ok(ref mut o) => o.try_for_each(|en| {
                en.wrap_err_with(|| eyre!("enter secret mount point error"))
                    .and_then(|d| {
                        match str::parse::<usize>(
                            d.file_name().to_string_lossy().to_string().as_str(),
                        ) {
                            Err(e) => Err(eyre!("parse mount point generation err: {}", e)),
                            Ok(res) => {
                                debug!("found mountpoint generation {}", res);
                                if res >= max {
                                    max = res + 1;
                                }
                                Ok(())
                            }
                        }
                    })
            }),
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
                .inspect(|p| {
                    fs::set_permissions(p, Permissions::from_mode(0o751))
                        .wrap_err(eyre!("set permission"))
                        .expect("set permission");
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

        if !self.templates.is_empty() {
            info!("start deploy templates");
            use sha2::{Digest, Sha256};

            let get_hashed_id = |s: &profile::Secret| -> Vec<u8> {
                let mut hasher = Sha256::new();
                hasher.update(s.id.as_str());
                hasher.finalize().to_vec()
            };

            // new map with sha256 hashed secret id str as key, ctx as value
            let hashstr_ctx_map: HashMap<Vec<u8>, &Vec<u8>> = plain_map
                .inner_ref()
                .iter()
                .map(|(k, v)| (get_hashed_id(k), v))
                .collect();

            self.templates.clone().iter().for_each(|(_, t)| {
                let mut template = t.content.clone();
                let hashstrs_of_it = t.parse_hash_str_list().expect("parse template");

                let trim_the_insertial = t.trim;

                hashstr_ctx_map
                    .iter()
                    .filter(|(k, _)| hashstrs_of_it.contains(k))
                    .for_each(|(k, v)| {
                        // render and insert
                        trace!("template before process: {}", template);

                        let raw_composed_insertial = String::from_utf8_lossy(v).to_string();

                        let insertial = if trim_the_insertial {
                            raw_composed_insertial.trim()
                        } else {
                            raw_composed_insertial.as_str()
                        };

                        template = template.replace(
                            format!("{{{{ {} }}}}", hex::encode(k.as_slice())).as_str(),
                            insertial,
                        );
                    });

                deploy_to_fs(
                    SecBuf::<Plain>::new(template.into_bytes()),
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
