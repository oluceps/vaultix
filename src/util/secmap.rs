use std::{
    fmt,
    fs::File,
    io::{self, Read},
    iter,
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
    sync::{Arc, RwLock},
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
use nom::AsBytes;
use papaya::HashMap;
use std::marker::PhantomData;

use super::secbuf::{HostEnc, Plain, SecBuf};

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
    fn have(&self, p: &PathBuf) -> bool {
        for ip in self.inner_ref().pin().values() {
            if &ip.path == p {
                return true;
            }
        }
        false
    }
    pub fn all_host_cache_in_repo(&self, cache_dir: PathBuf) -> Vec<PathBuf> {
        let mut ret = Vec::new();

        self.inner_ref().pin().iter().for_each(|((_, y), _)| {
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
            a.pin().iter().fold(HashMap::new(), |acc, (x, y)| {
                y.pin().iter().for_each(|(n, i)| {
                    acc.pin().update_or_insert_with(
                        (*n, i.clone()),
                        |i| {
                            let mut prev = i.clone();
                            prev.extend([x.clone()]);
                            i.clone()
                        },
                        || vec![x.clone()],
                    );
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
        RencInst::<'_, InRepo>(
            self.0.pin().iter().fold(
                HashMap::builder()
                    .resize_mode(papaya::ResizeMode::Blocking)
                    .build(),
                |acc, ((x, _), z)| {
                    z.iter().for_each(|h| {
                        let hash = ctx.0.pin().get(x).expect("never").hash_with(h.recip());
                        let in_repo = {
                            let mut p: PathBuf = cache_dir.clone();
                            p.push(h.0);
                            p.push(hash.to_string());
                            SecPathBuf::<InRepo>::new(p)
                        };
                        acc.pin().insert((*x, h.clone()), in_repo);
                    });
                    acc
                },
            ),
        )
    }

    pub fn build_instore(self) -> RencInst<'a, InStore> {
        RencInst::<'_, InStore>(
            self.0
                .pin()
                .iter()
                .fold(HashMap::new(), |acc, ((x, y), z)| {
                    z.iter().for_each(|h| {
                        acc.pin().insert((*x, h.clone()), y.clone());
                    });
                    acc
                }),
        )
    }
}
impl<'a> RencInst<'a, InStore> {
    /// return self but processed the path to produce in-store cache/host/[hash] map
    pub fn renced_stored(self, ctx: &RencCtx<'a, AgeEnc>, host_cache_stored: PathBuf) -> Self {
        self.inner()
            .pin()
            .into_iter()
            .map(|((x, y), _)| {
                let mut dir = host_cache_stored.clone();
                let sec_hash = ctx
                    .inner_ref()
                    .pin()
                    .get(x)
                    .expect("must have")
                    .hash_with(y.recip())
                    .to_string();

                dir.push(sec_hash);

                ((*x, y.clone()), SecPath::new(dir))
            })
            .collect::<HashMap<(&'a Secret, HostInfo<'a>), SecPathBuf<InStore>>>()
            .into()
    }
    /// read secret file
    pub fn bake_decrypted(self, ident: Box<dyn Identity>) -> Result<HashMap<&'a Secret, Vec<u8>>> {
        let the = self.inner();
        let the_ref = the.pin();
        the_ref
            .into_iter()
            .map(|(k, v)| {
                v.read_buffer().map(|b| {
                    (
                        k.0,
                        SecBuf::<HostEnc>::from(b)
                            .decrypt(ident.as_ref())
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
        self.inner_ref()
            .pin()
            .keys()
            .map(|(_, v)| v)
            .try_for_each(|h| {
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
                    std::fs::remove_file(p)
                        .with_context(|| eyre!("cleaning old renc file error"))?;
                }
                Ok(())
            })
    }

    /// retain non exist path
    pub fn retain_noexist(&self) {
        self.inner_ref().pin().retain(|_, v| !v.path.exists())
    }

    pub fn makeup(&self, ctx: &RencCtx<'a, AgeEnc>, ident: Box<dyn Identity>) -> Result<()> {
        let key: Rc<dyn Identity> = Rc::from(ident);

        let material = self.inner_ref();
        let mat_ref = material.pin();

        let the_map: Arc<HashMap<&SecPathBuf<InRepo>, RwLock<SecBuf<Plain>>>> = Arc::new(
            mat_ref
                .iter()
                .map(|((sec, _), path)| {
                    ctx.inner_ref()
                        .pin()
                        .get(sec)
                        .with_context(|| eyre!("encrypted buf not found"))
                        .and_then(|buf_agenc| buf_agenc.decrypt(key.clone().as_ref()))
                        .and_then(|b| Ok((path, RwLock::new(b))))
                })
                .try_collect()?,
        );

        use std::io::Write;

        std::thread::scope(|s| {
            for ((_, host), inrepo_path) in mat_ref.iter() {
                let dst_ctt_map = the_map.clone();

                let host_ssh_recip: Box<dyn Recipient + Send> =
                    RawRecip::from(String::from_str(host.recip()).unwrap())
                        .try_into()
                        .unwrap();

                debug!("rencrypt [{}]", inrepo_path.path.display());
                std::fs::create_dir_all(inrepo_path.path.parent().expect("must have")).unwrap();

                s.spawn(move || {
                    let mut target_file = std::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(inrepo_path.path.clone())
                        .expect("yes");
                    let dst_ctt_map_ref = dst_ctt_map.pin();
                    let buf = dst_ctt_map_ref.get(inrepo_path).expect("").read().unwrap();
                    let ctt = buf
                        .clone()
                        .encrypt(iter::once(host_ssh_recip.as_ref()))
                        .unwrap();

                    target_file.write_all(ctt.inner().as_bytes()).unwrap();
                });
            }
        });
        Ok(())
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
        let m = HashMap::new();
        for i in iter.into_iter() {
            m.pin().insert(i.0, i.1);
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
        let m = HashMap::new();
        for i in iter.into_iter() {
            m.pin().insert(i.0, i.1);
        }
        Self(m)
    }
}
