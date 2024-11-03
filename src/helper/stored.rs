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
            impl FromIterator<(profile::Secret, $t)> for SecMap<$t> {
                fn from_iter<I: IntoIterator<Item = (profile::Secret, $t)>>(iter: I) -> Self {
                    let map = HashMap::from_iter(iter);
                    SecMap(map)
                }
            }
        )*
    };
}

impl_from_iterator_for_secmap!(Vec<u8>, blake3::Hash, UniPath);

#[derive(Debug, Clone)]
pub struct SecMap<P>(HashMap<profile::Secret, P>);

impl<T> SecMap<T> {
    pub fn inner(self) -> HashMap<profile::Secret, T> {
        self.0
    }
}
impl<T> SecPath<PathBuf, T> {
    pub fn calc_hash(&self, host_ssh_recip: String) -> Result<blake3::Hash> {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.read_buffer()?.as_slice());
        hasher.update(host_ssh_recip.as_bytes());
        Ok(hasher.finalize())
    }
}

impl<T> SecMap<SecPath<PathBuf, T>> {
    /// read secret file
    pub fn bake_ctx(self) -> Result<SecMap<Vec<u8>>> {
        self.inner()
            .into_iter()
            // TODO: reduce read
            .map(|(k, v)| v.read_buffer().and_then(|b| Ok((k, b))))
            .try_collect::<SecMap<Vec<u8>>>()
    }
}

impl SecMap<SecPath<PathBuf, InStore>> {
    pub fn from(secrets: SecretSet) -> Self {
        let res = secrets
            .into_values()
            .into_iter()
            .map(|s| {
                let secret_path = SecPath::<_, InStore>::new(PathBuf::from(s.file.clone()));
                (s, secret_path)
            })
            .collect();
        SecMap::<SecPath<PathBuf, InStore>>(res)
    }
    /// pass storageDirStore in
    pub fn renced(self, per_host_dir: PathBuf, host_pubkey: String) -> Self {
        let res = self
            .inner()
            .into_iter()
            .map(|(k, v)| {
                let mut dir = per_host_dir.clone();
                let sec_path = v;
                let sec_hash = sec_path
                    .read_buffer()
                    .and_then(|b| {
                        let mut hasher = blake3::Hasher::new();
                        hasher.update(b.as_slice());
                        hasher.update(host_pubkey.as_bytes());
                        let hash_final = hasher.finalize();
                        Ok(hash_final.to_string())
                    })
                    .expect("hash");
                dir.push(sec_hash);

                let renced_in_per_host_dir = dir;
                (k, SecPath::new(renced_in_per_host_dir))
            })
            .collect::<HashMap<profile::Secret, SecPath<PathBuf, InStore>>>();
        SecMap::<SecPath<PathBuf, InStore>>(res)
    }
}

#[derive(Debug, Clone)]
pub struct UniPath {
    store: SecPath<PathBuf, InStore>,
    real: SecPath<PathBuf, InCfg>,
}

impl UniPath {
    pub fn new(store: SecPath<PathBuf, InStore>, real: SecPath<PathBuf, InCfg>) -> Self {
        UniPath { store, real }
    }
}

pub struct Renc {
    pub map: SecMap<UniPath>,
    host_dir: PathBuf,
    host_recip: String,
}
impl Renc {
    pub fn new(secrets: SecretSet, host_dir: PathBuf, host_recip: String) -> Self {
        let p2 = SecMap::<SecPath<PathBuf, InStore>>::from(secrets);
        let p1 = SecMap::<SecPath<PathBuf, InCfg>>::from(
            p2.clone(),
            host_dir.clone(),
            host_recip.clone(),
        )
        .inner();
        let p2 = p2.inner();

        let mut merged_map = HashMap::new();

        p1.into_iter().for_each(|(key, vout)| {
            if let Some(vin) = p2.get(&key) {
                merged_map.insert(key, UniPath::new(vin.clone(), vout));
            }
        });
        Renc {
            map: SecMap(merged_map),
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
                let enc_hash = v.store.calc_hash(self.host_recip.clone()).ok()?;
                let mut renc_path = self.host_dir.clone();
                renc_path.push(enc_hash.to_string());
                if renc_path.exists() {
                    return None;
                }
                Some((k, v))
            })
            .collect::<HashMap<profile::Secret, UniPath>>();
        Renc {
            map: SecMap(ret),
            host_dir: self.host_dir,
            host_recip: self.host_recip,
        }
    }
}

impl SecMap<UniPath> {
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
                // Ok(())
            })
            .collect()
    }
}

impl SecMap<SecPath<PathBuf, InCfg>> {
    fn from(
        value: SecMap<SecPath<PathBuf, InStore>>,
        host_dir: PathBuf,
        host_recip: String,
    ) -> Self {
        let res = value
            .inner()
            .into_iter()
            .map(|(k, v)| {
                let enc_hash = v.calc_hash(host_recip.clone()).expect("ok");
                let mut renc_path = host_dir.clone();
                renc_path.push(enc_hash.to_string());
                (k, SecPath::<_, InCfg>::new(renc_path))
            })
            .collect::<HashMap<profile::Secret, SecPath<_, InCfg>>>();
        SecMap(res)
    }
}
