use serde;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct RepositoryConfig {
    pub offset: Option<usize>,
    pub authors: Option<HashMap<String, String>>,
    pub branches: Option<HashMap<String, String>>,
    #[serde(default)]
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
            allow_unnamed_heads: true,
            limit_high: None,
            path_prefix: None,
            branch_prefix: None,
            tag_prefix: None,
            prefix_default_branch: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PathRepositoryConfig {
    pub alias: Option<String>,
    pub path_hg: PathBuf,
    pub path_git: PathBuf,
    #[serde(default)]
    pub config: RepositoryConfig,
    pub merged_branches: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MultiConfig {
    pub path_git: PathBuf,
    pub repositories: Vec<PathRepositoryConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RepositorySavedState {
    OffsetedRevision(usize),
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn repository_saved_state_to_toml() {
        let expected = "type = \"OffsetedRevision\"\nvalue = 100\n";
        let result = toml::to_string(&super::RepositorySavedState::OffsetedRevision(100)).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn multiconfig_read_from_toml() {
        let src = r#"path_git = "000_git"

[[repositories]]
path_hg = "001_hg"
path_git = "001_git"

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
alias = "another_002"
path_hg = "002_hg"
path_git = "002_git"

"#;
        let result: super::MultiConfig = toml::from_str(src).unwrap();
        assert_eq!(2, result.repositories.len());

        let repository = &result.repositories[0];
        assert_eq!(PathBuf::from("001_hg"), repository.path_hg);
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

        assert_eq!(PathBuf::from("002_hg"), result.repositories[1].path_hg);
    }
}
