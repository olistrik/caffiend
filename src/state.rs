//! The products seen so far, kept in a JSON file so a restart does not announce everything again.

use std::{collections::BTreeMap, fs, io::ErrorKind, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::product::Product;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    pub products: BTreeMap<u64, Product>,
}

impl State {
    /// `None` when there is no state yet, which means this is the first run.
    pub fn load(path: &Path) -> Result<Option<Self>> {
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", path.display()))
                .map(Some),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(self).context("serializing state")?;
        fs::write(&tmp, text).with_context(|| format!("writing {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
    }
}
