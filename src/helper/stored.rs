use std::{
    collections::HashMap,
    fmt,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    rc::Rc,
};

use age::{Identity, Recipient};
use eyre::Context;
use spdlog::{debug, trace};

use crate::{
    helper::secret_buf::{AgeEnc, SecBuf},
    profile::{self, SecretSet},
};
use eyre::{eyre, Result};
use std::marker::PhantomData;

use super::secret_buf::HostEnc;

#[derive(Debug, Clone)]
pub struct SecPath<P: AsRef<Path>, T> {
    pub path: P,
    _marker: PhantomData<T>,
}

impl<P: AsRef<Path>, T> fmt::Display for SecPath<P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.as_ref().display())
    }
}

#[derive(Debug, Clone)]
pub struct InStore;
#[derive(Debug, Clone)]
pub struct InCfg;

type SecPBWith<A> = SecPath<PathBuf, A>;

pub trait GetSec {
    fn read_buffer(&self) -> Result<Vec<u8>>;
    fn open_file(&self) -> Result<File>;
}

impl<P, T> SecPath<P, T>
where
    P: AsRef<Path>,
{
    pub fn new(path: P) -> Self {
        SecPath {
            path,
            _marker: PhantomData,
        }
    }

    pub fn calc_hash(&self, host_ssh_recip: &str) -> Result<blake3::Hash> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.read_buffer()?.as_slice());
        hasher.update(host_ssh_recip.as_bytes());
        Ok(hasher.finalize())
    }
}

impl<P, T> GetSec for SecPath<P, T>
where
    P: AsRef<Path>,
{
    fn open_file(&self) -> Result<File> {
        trace!("opening {}", &self);
        File::open(&self.path).wrap_err_with(|| eyre!("open secret file error"))
    }

    fn read_buffer(&self) -> Result<Vec<u8>> {
        let mut f = self.open_file()?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)
            .wrap_err_with(|| eyre!("read secret file error"))?;
        Ok(buffer)
    }
}

macro_rules! impl_from_iterator_for_secmap {
    ($($t:ty),*) => {
        $(
            impl<'a> FromIterator<(&'a profile::Secret, $t)> for SecMap<'a,$t> {
                fn from_iter<I: IntoIterator<Item = (&'a profile::Secret, $t)>>(iter: I) -> Self {
                    let map = HashMap::from_iter(iter);
                    SecMap(map)
                }
            }
        )*
    };
}
impl_from_iterator_for_secmap!(Vec<u8>, blake3::Hash, UniPath, SecBuf<HostEnc>);

macro_rules! impl_from_for_secmap {
    ($($t:ty),*) => {
        $(
            impl<'a> From<HashMap<&'a profile::Secret, SecPBWith<$t>>>
                for SecMap<'a, SecPBWith<$t>>
            {
                fn from(map: HashMap<&'a profile::Secret, SecPBWith<$t>>) -> Self {
                    SecMap::<SecPBWith<$t>>(map)
                }
            }
        )*
    };
}

impl_from_for_secmap!(InCfg, InStore);

#[derive(Debug, Clone)]
pub struct SecMap<'a, P>(HashMap<&'a profile::Secret, P>);

impl<'a, T> SecMap<'a, T> {
    pub fn inner(self) -> HashMap<&'a profile::Secret, T> {
        self.0
    }
    pub fn inner_ref(&self) -> &HashMap<&'a profile::Secret, T> {
        &self.0
    }
}

impl<T> SecMap<'_, SecPath<PathBuf, T>> {
    fn have(&self, p: &PathBuf) -> bool {
        for ip in self.inner_ref().values() {
            if &ip.path == p {
                return true;
            }
        }
        false
    }
}

impl<'a> SecMap<'a, SecPBWith<InStore>> {
    pub fn create(secrets: &'a SecretSet) -> Self {
        SecMap::<SecPBWith<InStore>>(
            secrets
                .values()
                .map(|s| {
                    let secret_path = SecPath::<_, InStore>::new(PathBuf::from(s.file.clone()));
                    (s, secret_path)
                })
                .collect(),
        )
    }

    /// return self but processed the path to produce in-store storageInStore/[hash] map
    pub fn renced_stored(self, per_host_dir: PathBuf, host_pubkey: &str) -> Self {
        self.inner()
            .into_iter()
            .map(|(k, v)| {
                let mut dir = per_host_dir.clone();
                let sec_path = v;
                let sec_hash = sec_path.calc_hash(host_pubkey).expect("meow").to_string();
                dir.push(sec_hash);

                let renced_in_per_host_dir = dir;
                (k, SecPath::new(renced_in_per_host_dir))
            })
            .collect::<HashMap<&profile::Secret, SecPBWith<InStore>>>()
            .into()
    }

    /// read secret file
    pub fn bake_ctx(self) -> Result<SecMap<'a, SecBuf<HostEnc>>> {
        self.inner()
            .into_iter()
            .map(|(k, v)| v.read_buffer().map(|b| (k, SecBuf::from(b))))
            .try_collect::<SecMap<SecBuf<HostEnc>>>()
    }
}

#[derive(Debug, Clone)]
pub struct UniPath {
    store: SecPBWith<InStore>,
    real: SecPBWith<InCfg>,
}

impl UniPath {
    pub fn new(store: SecPBWith<InStore>, real: SecPBWith<InCfg>) -> Self {
        UniPath { store, real }
    }
}

pub struct Renc<'a> {
    pub map: SecMap<'a, UniPath>,
    host_dir: PathBuf,
    host_recip: &'a str,
}

impl<'a> Renc<'a> {
    pub fn create(secrets: &'a SecretSet, host_dir: PathBuf, host_recip: &'a str) -> Self {
        let instore = SecMap::<SecPBWith<InStore>>::create(secrets);
        let incfg = SecMap::<SecPBWith<InCfg>>::create(secrets, host_dir.clone(), host_recip);
        incfg.clean_old(host_dir.clone()).expect("success");
        let map = incfg
            .inner()
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    UniPath::new(instore.inner_ref().get(k).expect("promise").clone(), v),
                )
            })
            .collect::<SecMap<UniPath>>();

        Renc {
            map,
            host_dir,
            host_recip,
        }
    }

    pub fn filter_exist(self) -> Self {
        let ret = self
            .map
            .inner()
            .into_iter()
            .filter_map(|(k, v)| {
                let enc_hash = v.store.calc_hash(self.host_recip).ok()?;
                let mut renc_path = self.host_dir.clone();
                renc_path.push(enc_hash.to_string());
                if renc_path.exists() {
                    return None;
                }
                Some((k, v))
            })
            .collect::<HashMap<&profile::Secret, UniPath>>();
        Renc {
            map: SecMap(ret),
            host_dir: self.host_dir,
            host_recip: self.host_recip,
        }
    }
}

impl SecMap<'_, UniPath> {
    pub fn makeup(self, recips: Vec<Rc<dyn Recipient>>, ident: &dyn Identity) -> Result<()> {
        self.inner().into_values().try_for_each(|sec_path| {
            let UniPath { store, real } = sec_path;
            use std::io::Write;

            trace!("re-encrypted output path {}", real.path.display());
            let enc_ctx = store.read_buffer().expect("read buffer in store err");
            // rencrypt
            let renc_ctx = SecBuf::<AgeEnc>::new(enc_ctx)
                .renc(ident, recips.clone())
                .expect("renc_ctx err");

            let mut target_file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(real.path.clone())?;

            target_file
                .write_all(renc_ctx.buf_ref())
                .wrap_err_with(|| eyre!("write renc file error"))
        })
    }
}

impl<'a> SecMap<'a, SecPBWith<InCfg>> {
    fn create(src: &'a SecretSet, host_dir: PathBuf, host_recip_str: &str) -> Self {
        let instore = SecMap::<SecPBWith<InStore>>::create(src);
        instore
            .inner()
            .into_iter()
            .map(|(k, v)| {
                let enc_hash = v.calc_hash(host_recip_str).expect("ok");
                let mut renc_path = host_dir.clone();
                renc_path.push(enc_hash.to_string());
                (k, SecPath::<_, InCfg>::new(renc_path))
            })
            .collect::<HashMap<&profile::Secret, SecPBWith<InCfg>>>()
            .into()
    }

    fn clean_old(&self, host_dir: PathBuf) -> Result<()> {
        let tobe_clean = fs::read_dir(host_dir)?.filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file() && !self.have(&path) {
                Some(path)
            } else {
                None
            }
        });

        for p in tobe_clean {
            debug!("cleaning old: {}", p.display());
            fs::remove_file(p).with_context(|| eyre!("cleaning old renc file error"))?
        }
        Ok(())
    }
}
