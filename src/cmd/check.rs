use eyre::Result;
use spdlog::error;

use crate::{
    helper::stored::{InStore, SecMap, SecPath},
    profile::Profile,
};

impl Profile {
    pub fn check(self) -> Result<()> {
        let s_p_map = SecMap::<SecPath<_, InStore>>::from(self.secrets).inner();

        s_p_map
            .into_values()
            .map(|p| {
                if !p.path.exists() {
                    error!("path {} not exist, try run renc", p.path.display());
                    return Err(eyre::eyre!("rencypted secret not in expected location",));
                }
                Ok(())
            })
            .collect()
    }
}
