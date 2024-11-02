use std::{
    collections::HashMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use eyre::{Context, ContextCompat};

use crate::profile::{self, Profile, SecretSet, Settings};
use eyre::{eyre, Result};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct SecretPath<P: AsRef<Path>, T> {
    path: P,
    _marker: PhantomData<T>,
}

pub struct InStore;
pub struct InCfg;

pub trait GetSec {
    fn read_buffer(&self) -> Result<Vec<u8>>;
    fn open_file(&self) -> Result<File>;
}

impl<P, T> SecretPath<P, T>
where
    P: AsRef<Path>,
{
    pub fn new(path: P) -> Self {
        SecretPath {
            path,
            _marker: PhantomData,
        }
    }
}

impl<P, T> GetSec for SecretPath<P, T>
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

impl FromIterator<(profile::Secret, Vec<u8>)> for SecretPathMap<Vec<u8>> {
    fn from_iter<I: IntoIterator<Item = (profile::Secret, Vec<u8>)>>(iter: I) -> Self {
        let map = HashMap::from_iter(iter);
        SecretPathMap(map)
    }
}

type HashWithCtx = (blake3::Hash, Vec<u8>);

impl FromIterator<(profile::Secret, HashWithCtx)> for SecretPathMap<HashWithCtx> {
    fn from_iter<I: IntoIterator<Item = (profile::Secret, HashWithCtx)>>(iter: I) -> Self {
        let map = HashMap::from_iter(iter);
        SecretPathMap(map)
    }
}

#[derive(Debug, Clone)]
pub struct SecretPathMap<P>(HashMap<profile::Secret, P>);

impl<T> SecretPathMap<T> {
    pub fn inner(self) -> HashMap<profile::Secret, T> {
        self.0
    }
}

impl<T> SecretPathMap<SecretPath<PathBuf, T>> {
    /// read secret file
    pub fn bake(self) -> Result<SecretPathMap<Vec<u8>>> {
        self.inner()
            .into_iter()
            .map(|(k, v)| v.read_buffer().and_then(|b| Ok((k, b))))
            .try_collect::<SecretPathMap<Vec<u8>>>()
    }

    /// hash of encrypted file content
    /// used in: renc, calc and compare
    ///          deploy, calc and find in store
    pub fn calc_renc(self, _host_pubkey: String) -> Result<SecretPathMap<HashWithCtx>> {
        self.bake().and_then(|h| {
            h.inner()
                .into_iter()
                .map(|(k, v)| {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(v.as_slice());
                    // hasher.update(host_pubkey.as_bytes());
                    let hash = hasher.finalize();
                    Ok((k, (hash, v)))
                })
                .try_collect::<SecretPathMap<HashWithCtx>>()
        })
    }
}

impl SecretPathMap<SecretPath<PathBuf, InStore>> {
    pub fn from(secrets: SecretSet) -> Self {
        let res = secrets
            .into_values()
            .into_iter()
            .map(|s| {
                let secret_path = SecretPath::<_, InStore>::new(PathBuf::from(s.file.clone()));
                (s, secret_path)
            })
            .collect();
        SecretPathMap::<SecretPath<PathBuf, InStore>>(res)
    }
}

impl SecretPathMap<SecretPath<PathBuf, InCfg>> {
    pub fn from(secrets: SecretSet) -> Self {
        let res = secrets
            .into_values()
            .into_iter()
            .map(|s| {
                let secret_path = SecretPath::<_, InCfg>::new(PathBuf::from(s.file.clone()));
                (s, secret_path)
            })
            .collect();
        SecretPathMap::<SecretPath<PathBuf, InCfg>>(res)
    }
}
