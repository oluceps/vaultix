use eyre::Result;
use spdlog::error;

use crate::profile::Profile;

// use super::stored_sec_path::SecretPathMap;

// impl Profile {
//     pub fn check(self) -> Result<()> {
//         let s_p_map = SecretPathMap::from_profile(&self).inner();

//         s_p_map.into_values().try_for_each(|p| {
//             if !p.clone().inner().exists() {
//                 error!("path {} not exist, try run renc", p.inner().display());
//                 return Err(eyre::eyre!("rencypted secret not in expected location",));
//             }
//             Ok(())
//         })
//     }
// }
