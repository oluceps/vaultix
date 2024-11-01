use std::{fs, str::FromStr};

use crate::profile::MasterIdentity;
use age::{x25519, Identity, IdentityFile, Recipient};
use eyre::{eyre, ContextCompat, Result};
use spdlog::{debug, error, info};

use super::callback::UiCallbacks;

// pub enum Parsed {
//     Native(ParsedNativeIdentity),
// }

pub struct ParsedIdentity {
    identity: Box<dyn Identity>,
    recipient: Box<dyn Recipient>,
}
// pub struct ParsedPluginIdentity {
//     identity: Box<dyn age::plugin::IdentityPluginV1<>>,
//     recipient: Box<dyn Recipient>,
// }
impl ParsedIdentity {
    pub fn new<I, R>(identity: I, recipient: R) -> Self
    where
        I: Identity + 'static,
        R: Recipient + 'static,
    {
        Self {
            identity: Box::new(identity),
            recipient: Box::new(recipient),
        }
    }
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
    pub fn parse(
        Self {
            identity,
            pubkey: _, // not required. trans from prv key so fast.
        }: &Self,
    ) -> Result<ParsedIdentity> {
        // TODO: more case matches
        // if age-plugin then get Recipient as recip
        if identity.is_empty() {
            return Err(eyre!("No identity found"));
        } else {
            macro_rules! create_entity {
                ($method:ident, $err_import:expr,  $err_context:expr) => {{
                    IdentityFile::from_file(identity.clone())
                        .map_err(|_| eyre!("import {} from file error", $err_import))?
                        .with_callbacks(UiCallbacks)
                        .$method()
                        .map_err(|e| eyre!("{}", e))?
                        .into_iter()
                        .next()
                        .with_context(|| $err_context)?
                }};
            }
            let ident = create_entity!(into_identities, "identity", "into identity fail");

            let recp = create_entity!(to_recipients, "recip", "into recip fail");

            return Ok(ParsedIdentity::from_exist(ident, recp));
        }
    }
}
