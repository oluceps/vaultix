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
            let ident_file_entry = {
                if let Ok(identity) = IdentityFile::from_file(identity.clone())
                    .map_err(|_| eyre!("import identity from file error"))
                    .and_then(|i| {
                        let identities = i
                            .with_callbacks(UiCallbacks)
                            .into_identities()
                            .map_err(|_| eyre!("into ident err"))?;
                        identities
                            .into_iter()
                            .next()
                            .with_context(|| "into identity fail")
                    })
                {
                    identity
                } else {
                    return Err(eyre!("parse identity error"));
                }
            };

            // return Ok(ParsedIdentity::new(
            //     ident_file_entry.clone(),
            //     the.to_public(),
            // ));
            // match ident_file_entry {
            //     IdentityFileEntry::Native(the) => {
            //         info!("use native");
            //         return Ok(ParsedIdentity::new(the.clone(), the.to_public()));
            //     }
            //     IdentityFileEntry::Plugin(the) => {
            //         info!("use plugin");
            //         debug!("{}", the.to_string());
            //         if let Ok(i) = age::plugin::Identity::from_str(the.to_string().as_str()) {
            //             if let Some(line) = fs::read_to_string(identity)
            //                 .expect("read file")
            //                 .lines()
            //                 .find(|line| line.contains("Recipient:"))
            //             {
            //                 if let Some(start_index) = line.find("Recipient:") {
            //                     let recipient = line[(start_index + "Recipient:".len())..].trim();
            //                     println!("Recipient: {}", recipient);
            //                     if let Ok(o) = age::plugin::Recipient::from_str(recipient) {
            //                         // return Ok(ParsedIdentity {
            //                         //     identity: Box::new(i),
            //                         //     recipient: Box::new(o),
            //                         // });
            //                     }
            //                 }
            //             } else {
            //                 println!("Recipient not found");
            //             }
            //         };

            //         // the
            //         // return Ok(ParsedIdentity::new(the.clone()));
            //     }
            // }

            Err(eyre!(""))

            // nya!
            // if let Err(ref e) = ident {
            //     error!("{}", e);
            // }

            // WARN: FOR TEST
            // let key = age::x25519::Identity::generate();

            // Ok(ParsedIdentity::new(ident.unwrap(), recip.ok()))
        }
    }
}
