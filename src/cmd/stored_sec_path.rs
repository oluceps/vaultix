use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    iter,
    path::{Path, PathBuf},
};

use age::Identity;
use eyre::{Context, ContextCompat};
use nom::Err;

use crate::profile::{self, Profile, SecretSet, Settings};
use eyre::{eyre, Result};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct SecPath<P: AsRef<Path>, T> {
    path: P,
    _marker: PhantomData<T>,
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

impl_from_iterator_for_secmap!(Vec<u8>, HashWithCtx, PathWithCtx);

pub struct HashWithCtx(blake3::Hash, Vec<u8>);

impl HashWithCtx {
    pub fn new(b: blake3::Hash, v: Vec<u8>) -> Self {
        HashWithCtx(b, v)
    }
    pub fn get_hash(&self) -> &blake3::Hash {
        &self.0
    }
    pub fn get_ctx(&self) -> &Vec<u8> {
        &self.1
    }
}

#[derive(Debug, Clone)]
pub struct SecMap<P>(HashMap<profile::Secret, P>);

impl<T> SecMap<T> {
    pub fn inner(self) -> HashMap<profile::Secret, T> {
        self.0
    }
}

impl<T> SecMap<SecPath<PathBuf, T>> {
    /// read secret file
    pub fn bake_ctx(self) -> Result<SecMap<Vec<u8>>> {
        self.inner()
            .into_iter()
            .map(|(k, v)| v.read_buffer().and_then(|b| Ok((k, b))))
            .try_collect::<SecMap<Vec<u8>>>()
    }

    /// hash of encrypted file content
    /// used in: renc, calc and compare
    ///          deploy, calc and find in store
    pub fn calc_renc(self, _host_pubkey: String) -> Result<SecMap<HashWithCtx>> {
        self.bake_ctx().and_then(|h| {
            h.inner()
                .into_iter()
                .map(|(k, v)| {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(v.as_slice());
                    // hasher.update(host_pubkey.as_bytes());
                    let hash = hasher.finalize();
                    Ok((k, HashWithCtx::new(hash, v)))
                })
                .try_collect::<SecMap<HashWithCtx>>()
        })
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
}

impl SecMap<SecPath<PathBuf, InCfg>> {
    pub fn from(secrets: SecretSet, storage_abs_cfg: PathBuf) -> Self {
        let res = secrets
            .into_values()
            .into_iter()
            .map(|s| {
                s.file
                    .clone()
                    .split_once('-') // '-'
                    .and_then(|(_, n)| Some(n)) // ,_,
                    .wrap_err_with(|| eyre!("something wrong with secret file name in store"))
                    .and_then(|file_n| {
                        let mut path = storage_abs_cfg.clone();
                        path.push(file_n);
                        let secret_path = SecPath::<_, InCfg>::new(path);
                        Ok((s, secret_path))
                    })
            })
            .try_collect()
            .expect("ok");
        SecMap::<SecPath<PathBuf, InCfg>>(res)
    }

    pub fn makeup<F>(self, target: Vec<profile::Secret>, host_pub: String, dec: F) -> Result<()>
    where
        F: Fn(&Vec<u8>) -> Result<Vec<u8>>,
    {
        let spm: HashMap<profile::Secret, SecPath<PathBuf, InCfg>> = self
            .inner()
            .into_iter()
            .filter(|(s, _)| target.contains(s))
            .collect();

        let map_path_with_ctx: SecMap<PathWithCtx> = SecMap::<SecPath<_, InCfg>>(spm).into();
        map_path_with_ctx
            .inner()
            .into_iter()
            .try_for_each(|(_, v)| {
                let target_path = v.get_path();
                let enc_ctx = v.get_ctx();
                // decrypt
                let dec_ctx = dec(enc_ctx)?;

                use std::io::Write;
                use std::str::FromStr;
                let recip_host_pubkey = age::ssh::Recipient::from_str(host_pub.as_str())
                    .map_err(|_| eyre!("add recipient from host pubkey fail"))?;

                let encryptor =
                    age::Encryptor::with_recipients(iter::once(&recip_host_pubkey as _))
                        .map_err(|_| eyre!("create encryptor err"))?;

                let mut renc_ctx = vec![];

                let mut writer = encryptor.wrap_output(&mut renc_ctx)?;

                writer.write_all(&dec_ctx[..])?;
                writer.finish()?;

                let mut target_file = fs::OpenOptions::new().write(true).open(target_path)?;

                target_file
                    .write_all(&renc_ctx)
                    .wrap_err_with(|| eyre!("write renc file error"))
            })
    }
}

#[derive(Debug, Clone)]
pub struct PathWithCtx(SecPath<PathBuf, InCfg>, Vec<u8>);

impl PathWithCtx {
    pub fn get_path(&self) -> &PathBuf {
        &self.0.path
    }
    pub fn get_ctx(&self) -> &Vec<u8> {
        &self.1
    }
}

impl From<SecMap<SecPath<PathBuf, InCfg>>> for SecMap<PathWithCtx> {
    fn from(value: SecMap<SecPath<PathBuf, InCfg>>) -> Self {
        value
            .inner()
            .into_iter()
            .filter_map(|(s, p)| {
                let mut f = p.open_file().ok()?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)
                    .wrap_err_with(|| eyre!("read secret file error"))
                    .ok()?;
                Some((s, PathWithCtx(p, buffer)))
            })
            .collect()
    }
}
// impl From<SecMap<PathWithCtx>> for
