#![allow(dead_code)]

use gym::db::Db;
use std::path::PathBuf;

/// A directory that vanishes with the test.
pub struct Scratch(PathBuf);

impl Scratch {
    pub fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "gym-test-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&p).expect("scratch directory");
        Self(p)
    }

    pub fn path(&self) -> &std::path::Path {
        &self.0
    }

    pub fn db(&self) -> String {
        self.0.join("gym.db").to_string_lossy().to_string()
    }

    /// A config file the binary can be pointed at with `FPR_GYM_CONFIG_PATH`.
    pub fn config(&self, remote: &str) -> String {
        let p = self.0.join("gym.toml");
        std::fs::write(
            &p,
            format!("db = \"{}\"\nremote = \"{}\"\n", self.db(), remote),
        )
        .expect("config file");
        p.to_string_lossy().to_string()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub async fn fresh(name: &str) -> (Scratch, Db) {
    let s = Scratch::new(name);
    let db = Db::create(&s.db()).await.expect("create a database");
    (s, db)
}

pub fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_gym")
}
