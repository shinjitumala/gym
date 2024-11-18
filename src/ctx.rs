use serde::Deserialize;
use std::{env, fs, path::PathBuf};

use crate::com::*;

#[derive(Deserialize, Clone)]
pub struct Cfg {
    pub db: String,
    pub repo: String,
}

#[derive(Clone)]
pub struct C {
    pub cfg: Cfg,
}

impl C {
    pub fn new() -> Res<Self> {
        let p = {
            let home = env::var("HOME").expect("Failed to get variable '$HOME'");
            let p = PathBuf::from(format!("{home}/.config/gym.json"));
            if p.exists() {
                p
            } else {
                let p = "CONFIG_PATH";
                let p = env::var(p).map_err(|_| format!("'{p}' not set."))?;
                PathBuf::from(p)
            }
        };

        let cfg: Cfg = serde_json::from_str(&fs::read_to_string(&p).map_err(|e| {
            format!(
                "Failed to read file '{}' because '{e}'",
                p.to_string_lossy()
            )
        })?)
        .map_err(|e| {
            format!(
                "Failed to parse rules from '{}' because '{e}'",
                p.to_string_lossy()
            )
        })?;

        let p = PathBuf::from(&cfg.db);
        if !p.exists() {
            Err(format!("Database path does not exist '{}'", cfg.db))?;
        }

        let p = PathBuf::from(&cfg.repo);
        if !p.exists() {
            Err(format!(
                "Data repository path does not exist '{}'",
                cfg.repo
            ))?;
        }

        Ok(C { cfg })
    }

    pub async fn db(&self) -> Res<Db> {
        Ok(Db::new(&self).await?)
    }
}
