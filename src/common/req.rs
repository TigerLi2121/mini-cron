use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, Clone)]
pub struct Page {
    pub page: usize,

    pub limit: usize,
}

impl Page {
    pub fn offset(&self) -> usize {
        (self.page - 1) * self.limit
    }
}
