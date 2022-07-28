use std::collections::HashMap;
use std::fs;
use std::io::BufReader;

use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json;

#[derive(Deserialize, Debug, Clone)]
pub(crate) struct RepoMap {
    #[serde(flatten)]
    repos: HashMap<String, RepoDefinition>,
}

impl RepoMap {
    pub fn lookup_by_name(&self, name: &str) -> Option<RepoDefinition> {
        self.repos.get(name).cloned()
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct RepoDefinition {
    pub repo: String,
    pub arch: String,
    pub platform: String,
    pub base_url: String,
    pub snapshots: HashMap<String, String>,
    pub latest_snapshot: String,
}

pub(crate) fn get_repository_map() -> &'static RepoMap {
    static MAPPING: Lazy<RepoMap> = Lazy::new(|| {
        let file = fs::File::open("repo_data_dump.json").expect("couldn't open file");

        let reader = BufReader::new(file);
        let mapping: RepoMap = serde_json::from_reader(reader).expect("couldn't deserialize");

        mapping
    });
    &*MAPPING
}
