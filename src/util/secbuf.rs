use std::fs::{OpenOptions, Permissions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::rc::Rc;
use std::{io::Read, iter, marker::PhantomData};

use age::{Identity, Recipient};
#[derive(Debug, Clone)]
pub struct AgeEnc;
#[derive(Debug, Clone)]
pub struct HostEnc;
#[derive(Debug, Clone)]
pub struct Plain;

#[derive(Debug, Clone)]
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
    pub fn inner(self) -> Vec<u8> {
        self.buf
    }

    pub fn hash_with(&self, host_ssh_recip: &str) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.buf);
        hasher.update(host_ssh_recip.as_bytes());
        hasher.finalize()
    }
}

use eyre::Result;
impl<T> SecBuf<T> {
    pub fn buf_ref(&self) -> &Vec<u8> {
        self.buf.as_ref()
    }
    pub fn decrypt(&self, ident: &dyn Identity) -> Result<SecBuf<Plain>> {
        let buffer = self.buf_ref();
        let decryptor = age::Decryptor::new(&buffer[..])?;

        let mut dec_content = vec![];
        let mut reader = decryptor.decrypt(iter::once(ident))?;
        let res = reader.read_to_end(&mut dec_content);
        if let Ok(b) = res {
            debug!("decrypted secret {} bytes", b);
        }
        Ok(SecBuf::new(dec_content))
    }
}

impl<T> From<Vec<u8>> for SecBuf<T> {
    fn from(value: Vec<u8>) -> Self {
        Self {
            buf: value,
            _marker: PhantomData,
        }
    }
}

impl SecBuf<AgeEnc> {
    pub fn renc(
        &self,
        ident: &dyn Identity,
        recips: Vec<Rc<dyn Recipient>>,
    ) -> Result<SecBuf<HostEnc>> {
        self.decrypt(ident).and_then(|d| d.encrypt(recips))
    }
}
use eyre::eyre;
use log::debug;

use super::set_owner_group;

impl SecBuf<Plain> {
    /// encrypt with host pub key, ssh key
    pub fn encrypt(self, recips: Vec<Rc<dyn Recipient>>) -> Result<SecBuf<HostEnc>> {
        let recips_iter = recips.iter().map(|boxed| boxed.as_ref() as &dyn Recipient);
        let encryptor = age::Encryptor::with_recipients(recips_iter)
            .map_err(|_| eyre!("create encryptor err"))?;

        let buf = self.buf_ref();
        let mut enc_content = vec![];

        let mut writer = encryptor.wrap_output(&mut enc_content)?;

        use std::io::Write;
        writer.write_all(buf)?;
        writer.finish()?;
        Ok(SecBuf::new(enc_content))
    }

    pub fn deploy_to_fs(
        &self,
        item: impl crate::profile::DeployFactor,
        dst: PathBuf,
    ) -> Result<()> {
        let mut the_file = {
            let mode = crate::parser::parse_octal_str(item.mode())
                .map_err(|e| eyre!("parse octal permission err: {}", e))?;
            let permissions = Permissions::from_mode(mode);

            let file = OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(dst)?;

            file.set_permissions(permissions)?;

            set_owner_group::set_owner_and_group(&file, item.owner(), item.group())?;

            file
        };
        the_file.write_all(self.buf_ref())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Write, str::FromStr};

    use super::*;

    #[test]
    fn test_renc() {
        let key = age::x25519::Identity::generate();
        let pubkey = key.to_public();

        let plaintext = b"Hello world!";

        // Encrypt the plaintext to a ciphertext...
        let encrypted = {
            let encryptor = age::Encryptor::with_recipients(iter::once(&pubkey as _))
                .expect("we provided a recipient");

            let mut encrypted = vec![];
            let mut writer = encryptor.wrap_output(&mut encrypted).expect("test");
            writer.write_all(plaintext).expect("test");
            writer.finish().expect("test");

            encrypted
        };

        // 0x01
        let new_recip_str = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
        let buf = SecBuf::<AgeEnc>::new(encrypted);
        let _ = buf
            .renc(
                &key as &dyn Identity,
                vec![
                    Rc::new(age::x25519::Recipient::from_str(new_recip_str).unwrap())
                        as Rc<dyn Recipient>,
                ],
            )
            .unwrap();
    }
}
