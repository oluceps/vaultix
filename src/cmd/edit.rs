use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    rc::Rc,
};

use crate::util::{
    secbuf::{AgeEnc, Plain, SecBuf},
    secmap::{GetSec, SecPath},
};
use crate::{
    parser::{
        identity::{ParsedIdentity, RawIdentity},
        recipient::RawRecip,
    },
    util::secmap::InRepo,
};

use age::Recipient;
use eyre::{eyre, Context, ContextCompat};
use log::info;
use nom::AsBytes;

use super::EditSubCmd;

pub fn edit(arg: EditSubCmd) -> eyre::Result<()> {
    let EditSubCmd {
        file,
        identity,
        recipient,
    } = arg;

    let id_parsed: ParsedIdentity = identity
        .with_context(|| eyre!("must provide identity to decrypt content"))
        .and_then(|i| RawIdentity::from(i).try_into())?;
    let recips = {
        let mut ret = recipient
            .into_iter()
            .map(|s| RawRecip::from(s).try_into().expect("convert"))
            .collect::<Vec<Rc<dyn Recipient>>>();
        ret.push(Rc::from(id_parsed.recipient));
        ret
    };

    if PathBuf::from(&file).exists() {
        let buf = SecPath::<String, InRepo>::new(file.clone())
            .read_buffer()
            .map(SecBuf::<AgeEnc>::from)?
            .decrypt(id_parsed.identity.as_ref())?
            .inner();
        let pre_hash = blake3::hash(buf.as_slice());

        let edited_buf_encrypted = {
            let edited = edit::edit(buf)?;

            if blake3::hash(edited.as_bytes()) == pre_hash {
                info!("file unchange");
                return Ok(());
            }

            SecBuf::<Plain>::new(edited.into_bytes())
                .encrypt(recips)?
                .inner()
        };
        let mut file = OpenOptions::new().write(true).truncate(true).open(&file)?;

        file.write_all(edited_buf_encrypted.as_bytes())?;
        return Ok(());
    }

    let edited_buf_encrypted = {
        let edited = edit::edit(vec![])?;

        SecBuf::<Plain>::new(edited.into_bytes())
            .encrypt(recips)?
            .inner()
    };

    let mut target_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file)?;

    target_file
        .write_all(&edited_buf_encrypted)
        .wrap_err_with(|| eyre!("write renc file error"))?;

    info!("edited file written");

    Ok(())
}
