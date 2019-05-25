use serde;
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
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

#[derive(Debug, Deserialize, Serialize, Default, PartialEq)]
pub struct PathRepositoryConfig {
    pub alias: Option<String>,
    pub path_hg: PathBuf,
    pub path_git: PathBuf,
    #[serde(default)]
    pub config: RepositoryConfig,
    pub merged_branches: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct MultiConfig {
    pub path_git: PathBuf,
    pub repositories: Vec<PathRepositoryConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "value")]
pub enum RepositorySavedState {
    OffsetedRevision(usize, usize),
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    #[test]
    fn repository_saved_state_to_toml() {
        let expected = "type = \"OffsetedRevision\"\nvalue = [100, 200]\n";
        let result =
            toml::to_string(&super::RepositorySavedState::OffsetedRevision(100, 200)).unwrap();
        assert_eq!(expected, result);
    }

    #[test]
    fn singleconfig_read_from_toml() {
        let src = include_str!("../examples/single.toml");
        let result: super::RepositoryConfig = toml::from_str(src).unwrap();
        assert_eq!(
            result,
            super::RepositoryConfig {
                allow_unnamed_heads: true,
                offset: Some(1000),
                path_prefix: Some("prefix1".into()),
                tag_prefix: Some("prefix2-".into()),
                branch_prefix: Some("prefix3-".into()),
                authors: Some(
                    vec![
                        ("aaa 1".into(), "Bbb <bbb@company.xyz>".into()),
                        ("aaa".into(), "Bbb <bbb@company.xyz>".into()),
                        ("ccc".into(), "Qqq <qqq@another.dom>".into()),
                        ("My <my_typo@wrong.xyz>".into(), "My <my@normal.xyz>".into()),
                    ]
                    .into_iter()
                    .collect()
                ),
                branches: Some(
                    vec![
                        ("anotherhg".into(), "othergit".into()),
                        ("branch in hg".into(), "branch-in-git".into()),
                    ]
                    .into_iter()
                    .collect()
                ),
                ..Default::default()
            }
        )
    }

    #[test]
    fn multiconfig_read_from_toml() {
        let src = include_str!("../examples/multi.toml");
        let result: super::MultiConfig = toml::from_str(src).unwrap();

        assert_eq!(
            result,
            super::MultiConfig {
                path_git: "000_git".into(),
                repositories: vec![
                    super::PathRepositoryConfig {
                        path_hg: "001_hg".into(),
                        path_git: "001_git".into(),
                        config: super::RepositoryConfig {
                            allow_unnamed_heads: true,
                            offset: Some(1000),
                            path_prefix: Some("prefix1".into()),
                            tag_prefix: Some("prefix2-".into()),
                            branch_prefix: Some("prefix3-".into()),
                            prefix_default_branch: true,
                            authors: Some(
                                vec![("aaa".into(), "Bbb <bbb@company.xyz>".into()),]
                                    .into_iter()
                                    .collect()
                            ),
                            branches: Some(
                                vec![("branch1".into(), "branch2".into()),]
                                    .into_iter()
                                    .collect()
                            ),
                            ..Default::default()
                        },
                        merged_branches: Some(
                            vec![("branch_in_git".into(), "branch2".into()),]
                                .into_iter()
                                .collect()
                        ),
                        ..Default::default()
                    },
                    super::PathRepositoryConfig {
                        alias: Some("another_002".into()),
                        path_hg: "002_hg".into(),
                        path_git: "002_git".into(),
                        config: super::RepositoryConfig {
                            ..Default::default()
                        },
                        merged_branches: Some(
                            vec![("branch_in_git".into(), "branch_in_hg".into()),]
                                .into_iter()
                                .collect()
                        ),
                    }
                ]
            }
        );
    }
}
