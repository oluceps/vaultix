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
use spdlog::trace;

use crate::{
    helper::secret_buf::{AgeEnc, SecBuf},
    profile::{self, SecretSet},
};
use eyre::{eyre, Result};
use std::marker::PhantomData;

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
impl_from_iterator_for_secmap!(Vec<u8>, blake3::Hash, UniPath);

macro_rules! impl_into_secmap_for_themap {
    ($($t:ty),*) => {
        $(
            impl<'a> Into<SecMap<'a, SecPBWith<$t>>>
                for HashMap<&'a profile::Secret, SecPBWith<$t>>
            {
                fn into(self) -> SecMap<'a, SecPBWith<$t>> {
                    SecMap::<SecPBWith<$t>>(self)
                }
            }
        )*
    };
}
impl_into_secmap_for_themap!(InCfg, InStore);

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

impl<'a, T> SecMap<'a, SecPath<PathBuf, T>> {
    /// read secret file
    pub fn bake_ctx(self) -> Result<SecMap<'a, Vec<u8>>> {
        self.inner()
            .into_iter()
            .map(|(k, v)| v.read_buffer().and_then(|b| Ok((k, b))))
            .try_collect::<SecMap<Vec<u8>>>()
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
        let map = SecMap::<SecPBWith<InCfg>>::create(&secrets, host_dir.clone(), host_recip)
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
                let enc_hash = v.store.calc_hash(&self.host_recip).ok()?;
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

impl<'a> SecMap<'a, UniPath> {
    pub fn makeup(self, recips: Vec<Rc<dyn Recipient>>, ident: &dyn Identity) -> Result<()> {
        self.inner()
            .into_iter()
            .map(|(_sec, sec_path)| {
                let UniPath { store, real } = sec_path;
                use std::io::Write;

                trace!("re-encrypted output path {}", real.path.display());
                let enc_ctx = store.read_buffer().expect("read buffer in store err");
                // rencrypt
                let renc_ctx = SecBuf::<AgeEnc>::new(enc_ctx)
                    .renc(ident, recips.first().expect("have").clone())
                    .expect("renc_ctx err");

                let mut target_file = fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(real.path.clone())?;

                target_file
                    .write_all(renc_ctx.buf_ref())
                    .wrap_err_with(|| eyre!("write renc file error"))
            })
            .collect()
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
}
