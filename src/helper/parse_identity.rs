use std::{fs, str::FromStr};

use crate::profile::MasterIdentity;
use age::{x25519, Identity, IdentityFile, Recipient};
use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info};

use super::callback::UiCallbacks;

pub struct ParsedIdentity {
    identity: Box<dyn Identity>,
    recipient: Box<dyn Recipient>,
}
impl ParsedIdentity {
    // pub fn new<I, R>(identity: I, recipient: R) -> Self
    // where
    //     I: Identity + 'static,
    //     R: Recipient + 'static,
    // {
    //     Self {
    //         identity: Box::new(identity),
    //         recipient: Box::new(recipient),
    //     }
    // }
    pub fn from_exist(identity: Box<dyn Identity>, recipient: Box<dyn Recipient>) -> Self {
        Self {
            identity,
            recipient,
        }
    }
    pub fn get_identity(&self) -> &Box<dyn Identity> {
        &self.identity
    }
    pub fn get_recipient(&self) -> &Box<dyn Recipient> {
        &self.recipient
    }
}

impl MasterIdentity {
    // get identiy and recipient from identity file,
    // only file that contains info of identity and recip supported at present
    // which is expected while using age generated identity
    pub fn parse(
        Self {
            identity,
            pubkey: _, // not required. trans from prv key so fast.
        }: &Self,
    ) -> Result<ParsedIdentity> {
        if identity.is_empty() {
            return Err(eyre!("No identity found"));
        } else {
            macro_rules! create {
                ($method:ident,  $err_context:expr) => {{
                    IdentityFile::from_file(identity.clone())
                        .map_err(|e| eyre!("import from file error: {}", e))?
                        .with_callbacks(UiCallbacks)
                        .$method()
                        .map_err(|e| eyre!("{}", e))?
                        .into_iter()
                        .next()
                        .with_context(|| $err_context)?
                }};
            }
            let ident = create!(into_identities, "into identity fail");

            let recip = create!(to_recipients, "into recip fail");

            return Ok(ParsedIdentity::from_exist(ident, recip));
        }
    }
}
