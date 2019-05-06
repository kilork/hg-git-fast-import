use serde::{Deserialize, Serialize};

use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize)]
pub struct Environment {
    pub no_clean_closed_branches: bool,
    pub authors: Option<HashMap<String, String>>,
    pub clean: bool,
    pub cron: bool,
    pub target_push: bool,
    pub target_pull: bool,
    pub source_pull: bool,
}
