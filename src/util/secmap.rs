use std::{
    collections::HashMap,
    fmt,
    fs::File,
    io::{self, Read},
    iter,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{
    cmd::renc::CompleteProfile,
    parser::recipient::RawRecip,
    profile::{self, Secret},
    util::secbuf::AgeEnc,
};
use age::{Identity, Recipient};
use eyre::{eyre, Result};
use eyre::{Context, ContextCompat};
use log::debug;
use std::marker::PhantomData;

use super::secbuf::{HostEnc, SecBuf};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SecPath<P: AsRef<Path>, T> {
    pub path: P,
    _marker: PhantomData<T>,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct InStore;
#[derive(Eq, Hash, PartialEq, Debug, Clone)]
pub struct InRepo;

type SecPathBuf<A> = SecPath<PathBuf, A>;

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
// identifier, recip
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct HostInfo<'a>(&'a str, &'a str);
impl<'a> HostInfo<'a> {
    pub fn id(&self) -> &'a str {
        self.0
    }
    pub fn recip(&self) -> &str {
        self.1
    }
}
#[derive(Debug, Clone)]
pub struct RencBuilder<'a>(HashMap<(&'a Secret, SecPathBuf<InStore>), Vec<HostInfo<'a>>>);

#[derive(Debug, Clone)]
pub struct RencInst<'a, W>(HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>>);

#[derive(Debug, Clone)]
pub struct RencCtx<'a, B>(HashMap<&'a Secret, SecBuf<B>>);

impl<'a, B> RencCtx<'a, B> {
    fn inner_ref(&self) -> &HashMap<&'a Secret, SecBuf<B>> {
        &self.0
    }
}

impl<'a> RencCtx<'a, AgeEnc> {
    pub fn create(material: &'a CompleteProfile) -> Self {
        let c = material
            .inner_ref()
            .iter()
            .flat_map(|x| x.secrets.values())
            .map(|i| {
                (
                    i,
                    SecPathBuf::<InStore>::from(i)
                        .read_buffer()
                        .map(SecBuf::new)
                        .expect("read store must success"),
                )
            })
            .collect();
        Self(c)
    }
}

impl<'a, W> RencInst<'a, W> {
    pub fn inner(self) -> HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        self.0
    }
    fn inner_ref(&self) -> &HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        &self.0
    }
    fn inner_ref_mut(&mut self) -> &mut HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<W>> {
        &mut self.0
    }
    fn have(&self, p: &PathBuf) -> bool {
        for ip in self.inner_ref().values() {
            if &ip.path == p {
                return true;
            }
        }
        false
    }
    pub fn all_host_cache_in_repo(&self, cache_dir: PathBuf) -> Vec<PathBuf> {
        let mut ret = Vec::new();

        self.inner_ref().iter().for_each(|((_, y), _)| {
            let mut c = cache_dir.clone();
            c.push(y.id());
            if !ret.contains(&c) {
                ret.push(c);
            }
        });
        ret
    }
}
impl<'a> RencBuilder<'a> {
    pub fn create(profiles: &'a CompleteProfile) -> Self {
        // hostinfo - <S-P(S)>
        let a: HashMap<HostInfo, HashMap<&Secret, SecPathBuf<InStore>>> = profiles
            .inner_ref()
            .iter()
            .map(|x| {
                let host_info = HostInfo(x.host_identifier(), x.host_pubkey());
                let s_ps: HashMap<&Secret, SecPathBuf<InStore>> = x
                    .secrets
                    .values()
                    .map(|n| (n, SecPathBuf::<InStore>::from(n)))
                    .collect();
                (host_info, s_ps)
            })
            .collect();

        // (S, P(S)) - [host]
        let b: HashMap<(&Secret, SecPathBuf<InStore>), Vec<HostInfo>> =
            a.iter().fold(HashMap::new(), |mut acc, (x, y)| {
                y.iter().for_each(|(n, i)| {
                    acc.entry((*n, i.clone()))
                        .and_modify(|i| i.push(x.clone()))
                        .or_insert_with(|| vec![x.clone()]);
                });
                acc
            });
        Self(b)
    }

    pub fn build_inrepo(
        self,
        ctx: &'a RencCtx<'a, AgeEnc>,
        cache_dir: PathBuf,
    ) -> RencInst<'a, InRepo> {
        RencInst::<'_, InRepo>(self.0.iter().fold(HashMap::new(), |mut acc, ((x, _), z)| {
            z.iter().for_each(|h| {
                let hash = ctx.0.get(x).expect("").hash_with(h.recip());
                let in_repo = {
                    let mut p: PathBuf = cache_dir.clone();
                    p.push(h.0);
                    p.push(hash.to_string());
                    SecPathBuf::<InRepo>::new(p)
                };
                acc.entry((*x, h.clone())).or_insert_with(|| in_repo);
            });
            acc
        }))
    }

    pub fn build_instore(self) -> RencInst<'a, InStore> {
        RencInst::<'_, InStore>(self.0.iter().fold(HashMap::new(), |mut acc, ((x, y), z)| {
            z.iter().for_each(|h| {
                acc.entry((*x, h.clone())).or_insert_with(|| y.clone());
            });
            acc
        }))
    }
}
impl<'a> RencInst<'a, InStore> {
    /// return self but processed the path to produce in-store cache/host/[hash] map
    pub fn renced_stored(self, ctx: &RencCtx<'a, AgeEnc>, host_cache_stored: PathBuf) -> Self {
        self.inner()
            .into_iter()
            .map(|((x, y), _)| {
                let mut dir = host_cache_stored.clone();
                let sec_hash = ctx
                    .inner_ref()
                    .get(x)
                    .expect("must have")
                    .hash_with(y.recip())
                    .to_string();

                dir.push(sec_hash);

                ((x, y), SecPath::new(dir))
            })
            .collect::<HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>>()
            .into()
    }
    /// read secret file
    pub fn bake_decrypted(self, ident: &dyn Identity) -> Result<HashMap<&'a Secret, Vec<u8>>> {
        self.inner()
            .into_iter()
            .map(|(k, v)| {
                v.read_buffer().map(|b| {
                    (
                        k.0,
                        SecBuf::<HostEnc>::from(b)
                            .decrypt(ident)
                            .expect("must")
                            .inner(),
                    )
                })
            })
            .try_collect::<HashMap<&'a Secret, Vec<u8>>>()
    }
}

impl<'a> RencInst<'a, InRepo> {
    pub fn clean_outdated(&self, cache_dir: PathBuf) -> Result<()> {
        let hosts: Vec<&HostInfo<'a>> = self.inner_ref().keys().map(|(_, v)| v).collect();

        hosts.iter().try_for_each(|h| {
            let host_cache_dir = {
                let mut c = cache_dir.clone();
                c.push(h.0);
                c
            };
            let dir = std::fs::read_dir(host_cache_dir);

            if let Err(ref e) = dir {
                if e.kind() == io::ErrorKind::NotFound {
                    return Ok(());
                }
            }

            let dir = dir?;

            let tobe_clean = dir.filter_map(|entry| {
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
                std::fs::remove_file(p).with_context(|| eyre!("cleaning old renc file error"))?;
            }
            Ok(())
        })
    }

    /// retain non exist path
    pub fn retain_noexist(&mut self) {
        self.inner_ref_mut().retain(|_, v| !v.path.exists())
    }

    pub fn makeup(&self, ctx: &RencCtx<'a, AgeEnc>, ident: &dyn Identity) -> Result<()> {
        self.inner_ref()
            .iter()
            .try_for_each(|((s, host), sec_path)| {
                use std::io::Write;

                debug!("rencrypt [{}]", sec_path.path.display());
                let buf = ctx
                    .inner_ref()
                    .get(s)
                    .with_context(|| eyre!("encrypted buf not found"))?;

                let host_ssh_recip: Box<dyn Recipient> =
                    RawRecip::from(String::from_str(host.recip())?).try_into()?;

                std::fs::create_dir_all(sec_path.path.parent().expect("must have"))?;
                let mut target_file = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(sec_path.path.clone())?;

                target_file
                    .write_all(
                        buf.renc(ident, iter::once(host_ssh_recip.as_ref()))?
                            .buf_ref(),
                    )
                    .with_context(|| eyre!("write renc file error"))
            })
    }
}

impl<'a> FromIterator<((&'a profile::Secret, HostInfo<'a>), SecPathBuf<InRepo>)>
    for RencInst<'a, InRepo>
{
    fn from_iter<
        T: IntoIterator<Item = ((&'a profile::Secret, HostInfo<'a>), SecPathBuf<InRepo>)>,
    >(
        iter: T,
    ) -> Self {
        let mut m = HashMap::new();
        for i in iter.into_iter() {
            m.insert(i.0, i.1);
        }
        Self(m)
    }
}
impl<'a> From<RencBuilder<'a>> for RencInst<'a, InStore> {
    fn from(value: RencBuilder<'a>) -> Self {
        value.build_instore()
    }
}
impl<P: AsRef<Path>, T> fmt::Display for SecPath<P, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.as_ref().display())
    }
}
impl<'a> From<&'a profile::Secret> for SecPathBuf<InStore> {
    fn from(value: &'a profile::Secret) -> Self {
        Self {
            path: value.file.clone().into(),
            _marker: PhantomData,
        }
    }
}
impl<'a> From<HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>> for RencInst<'a, InStore> {
    fn from(value: HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>) -> Self {
        Self(value)
    }
}
impl<'a> FromIterator<(&'a profile::Secret, SecBuf<AgeEnc>)> for RencCtx<'a, AgeEnc> {
    fn from_iter<T: IntoIterator<Item = (&'a profile::Secret, SecBuf<AgeEnc>)>>(iter: T) -> Self {
        let mut m = HashMap::new();
        for i in iter.into_iter() {
            m.insert(i.0, i.1);
        }
        Self(m)
    }
}
