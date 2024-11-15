use std::{
    collections::HashMap,
    fs::{self, OpenOptions, Permissions, ReadDir},
    io::{self, ErrorKind, Write},
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
    profile::{DeployFactor, HostKey, Profile},
};

use crate::helper::parse_recipient::RawRecip;
use age::Recipient;
use eyre::{eyre, Context, ContextCompat, Result};
use hex::decode;
use lib::extract_all_hashes;
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
    dst: PathBuf,
) -> Result<()> {
    let mut the_file = {
        let mode = crate::parser::parse_octal_str(item.mode())
            .map_err(|e| eyre!("parse octal permission err: {}", e))?;
        let permissions = Permissions::from_mode(mode);

        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(dst)?;

        file.set_permissions(permissions)?;

        helper::set_owner_group::set_owner_and_group(&file, item.owner(), item.group())?;

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
    pub fn get_decrypt_dir_path_for_user(&self) -> String {
        self.settings.decrypted_dir_for_user.to_string()
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
    pub fn deploy(self, early: bool) -> Result<()> {
        if self.secrets.is_empty() && self.templates.is_empty() {
            return Ok(());
        }
        let host_prv_key = &self.get_host_key_identity()?;

        let if_early = |i: &String| -> bool { self.need_by_user.contains(i) == early };

        let secrets_to_deploy = self.secrets.iter().filter(|i| if_early(i.0));

        let templates_map_iter = self.templates.iter().filter(|i| if_early(i.0));

        let plain_map: SecMap<Vec<u8>> =
            SecMap::<SecPath<_, InStore>>::from_iter(secrets_to_deploy.into_iter().map(|(_, v)| v))
                .renced_stored(
                    self.settings.cache_in_store.clone().into(),
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
        macro_rules! generate_dst {
            ($obj:expr, $settings:expr, $target_extract_dir:expr) => {{
                let default_path = {
                    let mut p: PathBuf = $settings.decrypted_dir.clone().into();
                    p.push($obj.name());
                    p
                };
                if PathBuf::from($obj.path()) == default_path {
                    let mut ret = $target_extract_dir.clone();
                    ret.push($obj.name());
                    ret
                } else {
                    if PathBuf::from($obj.path()).starts_with(&default_path) {
                        spdlog::warn!(
                            "extract to decryptedDir detected. recommend specify `name` instead of `path`."
                        );
                    }
                    info!("specified decrypt path detected");
                    $obj.path().into()
                }
            }};
        }

        // deploy general secrets
        plain_map
            .inner_ref()
            .iter()
            .map(|(n, c)| {
                let ctx = SecBuf::<Plain>::new(c.clone());
                let item = n as &dyn DeployFactor;
                let dst: PathBuf = generate_dst!(item, self.settings, target_extract_dir_with_gen);

                info!("secret {} -> {}", item.name(), dst.display(),);

                deploy_to_fs(ctx, *n, dst)
            })
            .for_each(|res| {
                if let Err(e) = res {
                    error!("{}", e);
                }
            });
        info!("finish secrets deployment");

        info!("start templates deployment");
        // new map with {{ hash }} String as key, ctx as value
        let hashstr_ctx_map: HashMap<&str, &Vec<u8>> = plain_map
            .inner_ref()
            .iter()
            .map(|(k, v)| {
                self.placeholder
                    .get_braced_from_id(k.id.as_str())
                    .wrap_err_with(|| {
                        eyre!("secrets corresponding to the template placeholder id not found")
                    })
                    .map(|i| (i, v))
                    .expect("found secret from placeholder id")
            })
            .collect();

        templates_map_iter
            .map(|(_, t)| {
                let mut template = t.content.clone();
                let hashstrs_of_it = t.parse_hash_str_list().expect("parse template");

                let trim_the_insertial = t.trim;

                hashstr_ctx_map
                    .iter()
                    .filter(|(k, _)| {
                        let mut v = Vec::new();
                        extract_all_hashes(k, &mut v);
                        hashstrs_of_it
                            // promised by nixos module
                            .contains(&decode(v.first().expect("only one")).expect("decoded"))
                    })
                    .for_each(|(k, v)| {
                        // render and insert
                        trace!("template before process: {}", template);

                        let raw_composed_insertial = String::from_utf8_lossy(v).to_string();

                        let insertial = if trim_the_insertial {
                            raw_composed_insertial.trim()
                        } else {
                            raw_composed_insertial.as_str()
                        };

                        template = template.replace(k, insertial);
                    });

                let item = &t as &dyn DeployFactor;

                let dst = generate_dst!(item, self.settings, target_extract_dir_with_gen);

                info!("template {} -> {}", item.name(), dst.display(),);
                deploy_to_fs(SecBuf::<Plain>::new(template.into_bytes()), t, dst)
            })
            .for_each(|res| {
                if let Err(e) = res {
                    error!("{}", e);
                }
            });

        info!("finish templates deployment");

        let symlink_dst = if early {
            self.get_decrypt_dir_path_for_user()
        } else {
            self.get_decrypt_dir_path()
        };
        info!(
            "link decrypted dir {} to {}",
            target_extract_dir_with_gen.display(),
            symlink_dst.as_str()
        );

        match std::fs::remove_file(&symlink_dst) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => Err(eyre!("{}", e))?,
            Ok(_) => {}
        }
        // link back to /run/vaultix*
        std::os::unix::fs::symlink(target_extract_dir_with_gen, symlink_dst)
            .wrap_err_with(|| "create symlink error")
    }
}
