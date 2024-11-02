use std::{
    collections::HashMap,
    fmt,
    fs::{self, File},
    io::Read,
    iter,
    path::{Path, PathBuf},
};

use eyre::{Context, ContextCompat};
use spdlog::info;

use crate::profile::{self, Profile, SecretSet, Settings};
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
        info!("opening {}", &self);
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

impl_from_iterator_for_secmap!(Vec<u8>, PathWithCtx, blake3::Hash);

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
        info!("bake_ctx");
        self.inner()
            .into_iter()
            // TODO: reduce read
            .map(|(k, v)| v.read_buffer().and_then(|b| Ok((k, b))))
            .try_collect::<SecMap<Vec<u8>>>()
    }

    /// hash of encrypted file content
    /// used in: renc, calc and compare
    ///          deploy, calc and find in store
    pub fn calc_renc(self, _host_pubkey: String) -> Result<SecMap<blake3::Hash>> {
        self.bake_ctx().and_then(|h| {
            h.inner()
                .into_iter()
                .map(|(k, v)| {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(v.as_slice());
                    // hasher.update(host_pubkey.as_bytes());
                    let hash = hasher.finalize();
                    Ok((k, hash))
                })
                .try_collect::<SecMap<blake3::Hash>>()
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
                let mut f = // TODO: reduce read
                    File::open(&s.file).wrap_err_with(|| eyre!("open secret file error"))?;
                let mut buffer = Vec::new();
                f.read_to_end(&mut buffer)
                    .wrap_err_with(|| eyre!("read secret file error"))?;
                let hash = blake3::hash(&buffer).to_string();
                let mut path = storage_abs_cfg.clone();
                path.push(hash);
                let secret_path = SecPath::<_, InCfg>::new(path);
                Ok::<(profile::Secret, SecPath<PathBuf, InCfg>), eyre::ErrReport>((s, secret_path))
            })
            .try_collect()
            .expect("ok");
        SecMap::<SecPath<PathBuf, InCfg>>(res)
    }

    pub fn makeup<F>(
        self,
        in_store_data: SecMap<SecPath<PathBuf, InStore>>,
        target: Vec<profile::Secret>,
        host_pub: String,
        dec: F,
    ) -> Result<()>
    where
        F: Fn(&Vec<u8>) -> Result<Vec<u8>>,
    {
        let spm: HashMap<profile::Secret, SecPath<PathBuf, InCfg>> = self
            .inner()
            .into_iter()
            .filter(|(s, _)| target.contains(s))
            .collect();

        in_store_data.inner().into_iter().try_for_each(|(s, v)| {
            let enc_ctx = v.read_buffer()?;
            let target_path = spm
                .get(&s)
                .cloned()
                .wrap_err_with(|| eyre!("getpatherror"))?
                .path;
            // decrypt
            let dec_ctx = dec(&enc_ctx)?;

            use std::io::Write;
            use std::str::FromStr;
            let recip_host_pubkey = age::ssh::Recipient::from_str(host_pub.as_str())
                .map_err(|_| eyre!("add recipient from host pubkey fail"))?;

            let encryptor = age::Encryptor::with_recipients(iter::once(&recip_host_pubkey as _))
                .map_err(|_| eyre!("create encryptor err"))?;

            let mut renc_ctx = vec![];

            let mut writer = encryptor.wrap_output(&mut renc_ctx)?;

            writer.write_all(&dec_ctx[..])?;
            writer.finish()?;

            let mut target_file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(target_path)?;

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

// impl From<SecMap<SecPath<PathBuf, InCfg>>> for SecMap<PathWithCtx> {
//     fn from(value: SecMap<SecPath<PathBuf, InCfg>>) -> Self {
//         value
//             .inner()
//             .into_iter()
//             .filter_map(|(s, p)| {
//                 let mut f = p.open_file().ok()?;
//                 let mut buffer = Vec::new();
//                 f.read_to_end(&mut buffer)
//                     .wrap_err_with(|| eyre!("read secret file error"))
//                     .ok()?;
//                 Some((s, PathWithCtx(p, buffer)))
//             })
//             .collect()
//     }
// }
// impl From<SecMap<PathWithCtx>> for
