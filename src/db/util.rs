use crate::com::*;
use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

pub fn from_toml<O: DeserializeOwned, P: AsRef<Path>>(p: P) -> Res<O> {
    let s = read_to_string(p.as_ref()).map_err(|e| {
        format!(
            "Failed to read '{}' becase '{e}'",
            p.as_ref().to_string_lossy()
        )
    })?;
    let o = toml::from_str(&s).map_err(|e| {
        format!(
            "Failed to parse '{}' as toml because '{e}'",
            p.as_ref().to_string_lossy()
        )
    })?;
    Ok(o)
}
pub fn to_toml<I: Serialize + Debug, P: AsRef<Path>>(i: &I, p: P) -> Res<()> {
    let s = toml::to_string(i)
        .map_err(|e| format!("Failed to convert '{:#?}' to toml because '{e}'", i))?;
    write(p.as_ref(), s).map_err(|e| {
        format!(
            "Failed to write to '{}' because '{e}'.",
            p.as_ref().to_string_lossy()
        )
    })?;
    Ok(())
}
fn h2b<K: Clone + Ord, V: Clone>(h: &HashMap<K, V>) -> BTreeMap<K, V> {
    h.iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

pub fn s2t(s: &str) -> Res<DateTime<Local>> {
    Ok(DateTime::<Local>::from(
        DateTime::parse_from_rfc3339(s).map_err(|e| e.to_string())?,
    ))
}
pub fn t2s<T: TimeZone>(t: DateTime<T>) -> String {
    t.to_utc().to_rfc3339()
}

pub trait Timed {
    fn time(&self) -> &str;
}

#[derive(Clone, Default, Deserialize)]
pub struct Entries<I> {
    pub e: HashMap<usize, I>,
}
impl<I: Sized + DeserializeOwned + Serialize + Debug + Clone> Entries<I> {
    fn load<P: AsRef<Path>>(p: P) -> Res<Self> {
        from_toml(p)
    }
    fn save<P: AsRef<Path>>(&self, p: P) -> Res<()> {
        #[derive(Serialize, Debug)]
        struct S<I> {
            e: BTreeMap<usize, I>,
        }
        to_toml(&S { e: h2b(&self.e) }, p)
    }
    pub fn add(&mut self, i: I) -> usize {
        let id = self.e.len();
        self.e.insert(id, i);
        id
    }
}

#[derive(Clone, Default, Deserialize)]
pub struct FileDb<I> {
    p: PathBuf,
    pub e: Entries<I>,
}
impl<I: Sized + DeserializeOwned + Serialize + Debug + Clone> FileDb<I> {
    pub fn load<P: AsRef<Path>>(p: P) -> Res<Self> {
        Ok(Self {
            p: p.as_ref().to_path_buf(),
            e: Entries::load(p)?,
        })
    }
    pub fn save(&self) -> Res<()> {
        self.e.save(&self.p)
    }
}

#[derive(Clone)]
pub struct DirDb<I> {
    p: PathBuf,
    t: DirDbType,
    b: Entries<I>,
    m: usize,
}

fn files<P: AsRef<Path>>(p: P) -> Res<Vec<PathBuf>> {
    Ok(fs_read_dir(p)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_file())
        .collect())
}

#[derive(Clone)]
pub enum DirDbType {
    Day,
    Month,
}
impl<I: Sized + DeserializeOwned + Serialize + Debug + Clone + Timed + Default> DirDb<I> {
    pub fn load<P: AsRef<Path>>(p: P, t: DirDbType) -> Res<Self> {
        let mut max = usize::MIN;
        for f in files(&p)? {
            max = max.max(
                Entries::<I>::load(f)?
                    .e
                    .iter()
                    .map(|(id, _)| *id)
                    .max()
                    .unwrap_or(usize::MIN),
            );
        }
        Ok(Self {
            p: p.as_ref().to_owned(),
            t,
            b: Entries::default(),
            m: max,
        })
    }
    pub fn load_full(&mut self) -> Res<()> {
        files(&self.p)?
            .into_iter()
            .map(|e| Entries::<I>::load(e))
            .process_results(|i| self.b.e.extend(i.map(|e| e.e.into_iter()).flatten()))?;
        Ok(())
    }
    pub fn add(&mut self, i: I) -> usize {
        self.m = self.m.saturating_add(1);
        self.b.e.insert(self.m, i);
        self.m
    }
    fn timed(&self) -> Res<Vec<(DateTime<Utc>, usize, I)>> {
        self.b
            .e
            .iter()
            .map(|(id, e)| -> Res<_> {
                let t = e.time();
                let d = s2t(t)
                    .map_err(|x| format!("Failed to parse date from '{e:#?}' because '{x}'"))?;
                Ok((d.to_utc(), *id, e.to_owned()))
            })
            .process_results(|i| i.collect())
    }
    pub fn save(&self) -> Res<()> {
        use DirDbType::*;
        let timed = self.timed()?;
        let f2d = match self.t {
            Day => timed
                .into_iter()
                .map(|(k, id, v)| (k.format("%F").to_string(), (id, v)))
                .into_group_map(),
            Month => timed
                .into_iter()
                .map(|(k, id, v)| (k.format("%Y-%m").to_string(), (id, v)))
                .into_group_map(),
        };
        for (k, v) in f2d {
            let p = self.p.join(format!("{k}.toml"));
            let mut o = Entries::<I>::load(&p).unwrap_or_default();
            o.e.extend(v);
            o.save(p)?;
        }
        Ok(())
    }
    pub fn find<P: FnMut(&(usize, I)) -> bool + Copy>(&self, p: P) -> Res<Vec<(usize, I)>> {
        Ok(files(&self.p)?
            .into_iter()
            .map(|e| Entries::<I>::load(e))
            .process_results(|i| i.map(|e| e.e.into_iter().filter(p)).flatten().collect())?)
    }
    pub fn find_mut<P: FnMut(&(usize, I)) -> bool + Copy>(
        &mut self,
        p: P,
    ) -> Res<Vec<(usize, &mut I)>> {
        let e = files(&self.p)?
            .into_iter()
            .map(|e| Entries::<I>::load(e))
            .process_results(|i| i.map(|e| e.e.into_iter().filter(p)).flatten().collect_vec())?;
        self.b.e.extend(e.clone());
        Ok(self
            .b
            .e
            .iter_mut()
            .filter(|(id, _)| e.iter().any(|(oid, _)| *oid == **id))
            .map(|(id, i)| (*id, i))
            .collect())
    }
}
