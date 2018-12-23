extern crate serde_derive;

use serde;
use serde_derive::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub struct RepositoryConfig {
    pub offset: Option<usize>,
    pub authors: Option<HashMap<String, String>>,
    pub branches: Option<HashMap<String, String>>,
    pub allow_unnamed_heads: bool,
    #[serde(skip_deserializing)]
    pub limit_high: Option<usize>,
    pub path_prefix: Option<String>,
    pub branch_prefix: Option<String>,
    pub tag_prefix: Option<String>,
    #[serde(default)]
    pub prefix_default_branch: bool,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        Self {
            offset: None,
            authors: None,
            branches: None,
            allow_unnamed_heads: false,
            limit_high: None,
            path_prefix: None,
            branch_prefix: None,
            tag_prefix: None,
            prefix_default_branch: false,
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct PathRepositoryConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub config: RepositoryConfig,
}

#[derive(Deserialize, Serialize)]
pub struct MultiConfig {
    pub repositories: Vec<PathRepositoryConfig>,
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
    use std::path::PathBuf;

    #[test]
    fn repository_saved_state_to_toml() {
        let expected = "type = \"OffsetedRevisionSet\"\nvalue = [100]\n";
        let result =
            toml::to_string(&super::RepositorySavedState::OffsetedRevisionSet(vec![100])).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn multiconfig_read_from_toml() {
        let src = r#"[[repositories]]
path = "001_hg"

[repositories.config]
allow_unnamed_heads = true
offset = 1000
path_prefix = 'prefix1'
tag_prefix = 'prefix2-'
branch_prefix = 'prefix3-'

[repositories.config.authors]
'aaa' = 'bbb'

[repositories.config.branches]
'branch1' = 'branch2'

[[repositories]]
path = "002_hg"

"#;
        let result: super::MultiConfig = toml::from_str(src).unwrap();
        assert_eq!(2, result.repositories.len());

        let repository = &result.repositories[0];
        assert_eq!(PathBuf::from("001_hg"), repository.path);
        assert!(repository.config.allow_unnamed_heads);
        assert_eq!(Some(1000), repository.config.offset);
        assert_eq!(Some("prefix1".into()), repository.config.path_prefix);
        assert_eq!(Some("prefix2-".into()), repository.config.tag_prefix);
        assert_eq!(Some("prefix3-".into()), repository.config.branch_prefix);
        let authors = &repository.config.authors.as_ref().unwrap();
        assert_eq!(authors.get(&"aaa".to_string()), Some(&String::from("bbb")));
        let branches = &repository.config.branches.as_ref().unwrap();
        assert_eq!(
            branches.get(&"branch1".to_string()),
            Some(&String::from("branch2"))
        );

        assert_eq!(PathBuf::from("002_hg"), result.repositories[1].path);
    }
}
