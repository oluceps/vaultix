use std::rc::Rc;

use age::{ssh, x25519, Recipient};
use eyre::eyre;
use serde::Deserialize;

// basically parse host pub key

#[derive(Debug, Deserialize, Clone)]
pub struct RawRecip(String);

impl From<String> for RawRecip {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl TryInto<Rc<dyn Recipient>> for RawRecip {
    type Error = eyre::ErrReport;
    fn try_into(self) -> Result<Rc<dyn Recipient>, Self::Error> {
        use std::str::FromStr;
        let recip_str = self.0.as_str();
        macro_rules! try_recipients {
            ($pub_str:expr, $($type:path),+) => {
                $(
                    if let Ok(o) = <$type>::from_str($pub_str) {
                        return Ok(Rc::new(o) as Rc<dyn Recipient>);
                    }
                )+
            };
        }
        try_recipients!(recip_str, ssh::Recipient, x25519::Recipient);
        Err(eyre!("incompatible recipient type"))
    }
}
