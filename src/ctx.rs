use crate::com::*;
use serde::Deserialize;
use std::{env, fs, future::Future, path::PathBuf, sync::OnceLock};
use tokio::runtime::{Builder, Runtime};

#[derive(Deserialize, Clone)]
pub struct Cfg {
    pub db: String,
    pub remote: String,
}

pub struct C {
    pub cfg: Cfg,
}

fn rt() -> &'static Runtime {
    static RT: OnceLock<Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a tokio runtime")
    })
}

impl C {
    pub fn new() -> Res<Self> {
        let p = {
            let home = env::var("HOME").expect("Failed to get variable '$HOME'");
            let p = PathBuf::from(format!("{home}/.config/gym.toml"));
            if p.exists() {
                p
            } else {
                let p = "CONFIG_PATH";
                let p = env::var(p).map_err(|_| format!("'{p}' not set."))?;
                PathBuf::from(p)
            }
        };

        let cfg: Cfg = toml::from_str(&fs::read_to_string(&p).map_err(|e| {
            format!(
                "Failed to read file '{}' because '{e}'",
                p.to_string_lossy()
            )
        })?)
        .map_err(|e| format!("Failed to parse '{}' because '{e}'", p.to_string_lossy()))?;

        let p = PathBuf::from(&cfg.db);
        if p.is_dir() {
            Err(format!(
                "'db' is '{}', a directory, but it must be the path to the database file.",
                cfg.db
            ))?;
        }
        if let Some(d) = p.parent() {
            if !d.as_os_str().is_empty() && !d.exists() {
                Err(format!(
                    "'db' is '{}', whose directory does not exist.",
                    cfg.db
                ))?;
            }
        }
        if cfg.remote.trim().is_empty() {
            Err("'remote' is empty; it is the rsync destination gym sync writes to.".to_string())?;
        }

        Ok(C { cfg })
    }

    pub fn block_on<F: Future>(&self, f: F) -> F::Output {
        rt().block_on(f)
    }

    pub async fn db(&self) -> Res<Db> {
        Db::open(&self.cfg.db).await
    }
}
