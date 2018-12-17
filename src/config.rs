extern crate serde_derive;

use serde_derive::{Deserialize, Serialize};
use serde;

use std::collections::HashMap;

#[derive(Deserialize, Serialize)]
pub struct RepositoryConfig {
    pub offset: Option<usize>,
    pub authors: Option<HashMap<String, String>>,
    pub branches: Option<HashMap<String, String>>,
    pub allow_unnamed_heads: bool,
    pub limit_high: Option<usize>,
    pub path_prefix: Option<String>,
    pub branch_prefix: Option<String>,
    pub tag_prefix: Option<String>,
    #[serde(default)]
    pub prefix_default_branch: bool,
}

#[derive(Deserialize, Serialize)]
pub struct Environment {
    pub no_clean_closed_branches: bool,
    pub authors: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RepositorySavedState {
    OffsetedRevisionSet(Vec<usize>),
}

#[cfg(test)]
mod tests {
    #[test]
    fn repository_saved_state_to_toml() {
        let expected = "type = \"OffsetedRevisionSet\"\nvalue = [100]\n";
        let result = toml::to_string(&super::RepositorySavedState::OffsetedRevisionSet(vec![100])).unwrap();
        assert_eq!(expected, result);
    }
}