use std::rc::Rc;
use std::{io::Read, iter, marker::PhantomData};

use age::{Identity, Recipient};
use spdlog::debug;

#[derive(Debug, Clone)]
pub struct AgeEnc;
#[derive(Debug, Clone)]
pub struct HostEnc;
#[derive(Debug, Clone)]
pub struct Plain;

pub struct SecBuf<T> {
    buf: Vec<u8>,
    _marker: PhantomData<T>,
}

impl<T> SecBuf<T> {
    pub fn new(i: Vec<u8>) -> Self {
        SecBuf {
            buf: i,
            _marker: PhantomData,
        }
    }
}

use eyre::Result;
impl<T> SecBuf<T> {
    pub fn buf_ref<'a>(&'a self) -> &'a Vec<u8> {
        self.buf.as_ref()
    }
    pub fn decrypt(&self, ident: &dyn Identity) -> Result<SecBuf<Plain>> {
        let buffer = self.buf_ref();
        let decryptor = age::Decryptor::new(&buffer[..])?;

        let mut dec_ctx = vec![];
        let mut reader = decryptor.decrypt(iter::once(ident))?;
        let res = reader.read_to_end(&mut dec_ctx);
        if let Ok(b) = res {
            debug!("decrypted secret {} bytes", b);
        }
        Ok(SecBuf::new(dec_ctx))
    }
}

impl SecBuf<AgeEnc> {
    pub fn renc(&self, ident: &dyn Identity, recips: Rc<dyn Recipient>) -> Result<SecBuf<HostEnc>> {
        self.decrypt(ident).and_then(|d| d.encrypt(vec![recips]))
    }
}
use eyre::eyre;
use spdlog::info;

impl SecBuf<Plain> {
    /// encrypt with host pub key, ssh key
    pub fn encrypt(self, recips: Vec<Rc<dyn Recipient>>) -> Result<SecBuf<HostEnc>> {
        let recips_iter = recips.iter().map(|boxed| boxed.as_ref() as &dyn Recipient);
        info!("things in recips iter {}", recips.len());
        let encryptor = age::Encryptor::with_recipients(recips_iter)
            .map_err(|_| eyre!("create encryptor err"))?;

        let buf = self.buf_ref();
        let mut enc_ctx = vec![];

        let mut writer = encryptor.wrap_output(&mut enc_ctx)?;

        use std::io::Write;
        writer.write_all(buf)?;
        writer.finish()?;
        Ok(SecBuf::new(enc_ctx))
    }
}
