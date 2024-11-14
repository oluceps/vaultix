use crate::profile::PlaceHolderSet;

impl PlaceHolderSet {
    pub fn get_braced_from_id(&self, id: impl AsRef<str>) -> Option<&str> {
        self.0.get(id.as_ref()).map(|i| i.as_str())
    }
}
