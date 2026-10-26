use crate::com::*;
use serde::Deserialize;
use std::{env, future::Future, path::PathBuf, sync::OnceLock};
use tokio::runtime::{Builder, Runtime};

/// Where the configuration path comes from, in the order it is tried: the
/// variable, then the file named below `$HOME`. The name is the one the other
/// `fpr` tools use, so a machine that sets one can set them all the same way.
pub const ENV: &str = "FPR_GYM_CONFIG_PATH";
pub const DEF: &str = ".config/gym.toml";

/// The configuration file to read, as a path.
pub fn config_path() -> Result<String, String> {
    if let Ok(p) = env::var(ENV) {
        return Ok(p);
    }
    let home = env::var("HOME").map_err(|_| "Failed to get variable '$HOME'.".to_string())?;
    Ok(format!("{home}/{DEF}"))
}

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

impl Ctx for C {
    fn new(path: &str) -> Result<Self, String> {
        let p = PathBuf::from(path);

        let cfg: Cfg = from_toml(&p)?;

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
}

impl C {
    /// A context that read no configuration.
    ///
    /// `--version` and `--help` are answered with it, because neither reads a
    /// setting: they report on the binary, and a binary that refuses to do
    /// that until a configuration file exists cannot answer the question a
    /// fresh install is asking.
    pub fn unconfigured() -> Self {
        C {
            cfg: Cfg {
                db: String::new(),
                remote: String::new(),
            },
        }
    }

    pub fn block_on<F: Future>(&self, f: F) -> F::Output {
        rt().block_on(f)
    }

    pub async fn db(&self) -> Res<Db> {
        Db::open(&self.cfg.db).await
    }
}
