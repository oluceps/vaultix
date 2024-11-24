use std::{
    iter,
    rc::Rc,
    str::FromStr,
    sync::{Arc, Mutex},
};

use age::{Identity, Recipient};
use dashmap::DashMap;
use log::{debug, error, info};
use nom::AsBytes;

use crate::{
    parser::recipient::RawRecip,
    util::{
        secbuf::{Plain, SecBuf},
        secmap::{InRepo, SecPathBuf},
    },
};

use super::{
    secbuf::AgeEnc,
    secmap::{RencCtx, RencInstance},
};

use eyre::{eyre, Context, ContextCompat, Result};

impl<'a> RencInstance<'a> {
    pub fn makeup(
        self,
        ctx: &RencCtx<'a, AgeEnc>,
        ident: Box<dyn Identity>,
    ) -> Result<Vec<String>> {
        let key: Rc<dyn Identity> = Rc::from(ident);

        let material = &self.inner().into_read_only();

        info!("decrypting...");

        let the_map: Arc<DashMap<&SecPathBuf<InRepo>, SecBuf<Plain>>> = Arc::new(
            material
                .values()
                .flatten()
                .map(|(k, v)| {
                    ctx.inner_ref()
                        .get(k)
                        .wrap_err_with(|| eyre!("encrypted buf not found"))
                        .and_then(|buf_agenc| buf_agenc.decrypt(key.clone().as_ref()))
                        .map(|b| (v, b))
                })
                .try_collect()?,
        );

        use std::io::Write;

        let res: Arc<Mutex<Vec<eyre::Result<&str>>>> = Arc::new(Mutex::new(Vec::new()));

        std::thread::scope(|s| {
            material.iter().for_each(|(h, v)| {
                debug!("rencrypting for [{}]", h.id());
                let res = res.clone();
                let dst_ctt_map = the_map.clone();

                let recip: Box<dyn Recipient + Send> = if let Ok(Ok(o)) =
                    String::from_str(h.recip())
                        .map(RawRecip::from)
                        .map(RawRecip::try_into)
                {
                    o
                } else {
                    res.lock()
                        .expect("doesn't matter now")
                        .push(Err(eyre!("parse host recipient fail")));
                    return;
                };

                s.spawn(move || {
                    for (_, inrepo_path) in v.iter() {
                        if let Err(e) = inrepo_path
                            .path
                            .parent()
                            .wrap_err_with(|| {
                                eyre!("cache file path has no parent, is this possible?")
                            })
                            .and_then(|i| {
                                std::fs::create_dir_all(i)
                                    .wrap_err_with(|| eyre!("create host cache dir in repo failed"))
                            })
                        {
                            res.lock().expect("doesn't matter now").push(Err(e));
                            return;
                        };
                        let mut target_file = if let Ok(o) = std::fs::OpenOptions::new()
                            .write(true)
                            .create(true)
                            .truncate(true)
                            .open(inrepo_path.path.clone())
                        {
                            o
                        } else {
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(eyre!("create file error")));
                            return;
                        };

                        let dst_ctt_map_ref = dst_ctt_map.clone();

                        let buf = dst_ctt_map_ref.get(inrepo_path).expect("should have");

                        let ctt = match buf.clone().encrypt(iter::once(recip.as_ref())) {
                            Ok(o) => o,
                            e @ Err(_) => {
                                res.lock()
                                    .expect("doesn't matter now")
                                    .push(e.map(|_| h.id()));
                                return;
                            }
                        };

                        if target_file.write_all(ctt.inner().as_bytes()).is_err() {
                            res.lock()
                                .expect("doesn't matter now")
                                .push(Err(eyre!("write cache file failed")))
                        };
                        res.lock().expect("thread work end").push(Ok(h.id()));
                    }
                });
            });
        });

        info!("finished");

        let last_res = res.lock().expect("");

        last_res.iter().for_each(|i| {
            if let Err(e) = i {
                error!("{}", e);
            }
        });

        Ok(last_res
            .iter()
            .filter(|i| i.is_ok())
            .map(|i| {
                if let Ok(o) = i {
                    String::from(*o)
                } else {
                    unreachable!()
                }
            })
            .collect())
    }
}
