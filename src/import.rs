use crate::{com::*, db::Db};
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
};

#[derive(Deserialize, Default)]
struct Entries<T> {
    #[serde(default)]
    e: BTreeMap<i64, T>,
}

#[derive(Deserialize, Default, Clone)]
struct Link {
    group: i64,
    #[serde(default = "one")]
    amount: f64,
}
fn one() -> f64 {
    1.0
}

#[derive(Deserialize, Default, Clone)]
struct TExercise {
    name: String,
    #[serde(default)]
    desc: String,
    #[serde(default)]
    muscle_groups: Vec<Link>,
}

#[derive(Deserialize, Default, Clone)]
struct TMuscleGroup {
    name: String,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TPlace {
    name: String,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TFood {
    name: String,
    calories: f64,
    #[serde(default)]
    protein: Option<f64>,
    #[serde(default)]
    fat: Option<f64>,
    #[serde(default)]
    carbohydrate: Option<f64>,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TSet {
    date: String,
    exercise: i64,
    load: f64,
    rep: f64,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TSession {
    date: String,
    place: i64,
    #[serde(default)]
    sets: Vec<i64>,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TWeight {
    date: String,
    kg: f64,
    #[serde(default)]
    bodyfat: Option<f64>,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default, Clone)]
struct TMeal {
    date: String,
    food: i64,
    #[serde(default = "one")]
    amount: f64,
    #[serde(default)]
    desc: String,
}

#[derive(Deserialize, Default)]
struct MapFile {
    #[serde(default)]
    map: BTreeMap<String, String>,
}

#[derive(Default, Debug)]
pub struct ImportReport {
    pub muscle_groups: usize,
    pub places: usize,
    pub exercises: usize,
    pub exercise_muscle_groups: usize,
    pub foods: usize,
    pub sessions: usize,
    pub sets: usize,
    pub weights: usize,
    pub meals: usize,
    pub supersets: usize,
    pub inferred: usize,
    pub superset_no_reps: Vec<String>,
    pub superset_unmapped: Vec<String>,
    pub superset_self: Vec<String>,
    pub duplicate_refs: Vec<String>,
}

impl ImportReport {
    pub fn to_lines(&self) -> String {
        let mut s = format!(
            "muscle groups    {}\nplaces           {}\nexercises        {}\nexercise groups  {}\nfoods            {}\nsessions         {}\nsets             {} ({} inferred, {} supersets)\nweigh-ins        {}\nmeals            {}",
            self.muscle_groups,
            self.places,
            self.exercises,
            self.exercise_muscle_groups,
            self.foods,
            self.sessions,
            self.sets,
            self.inferred,
            self.supersets,
            self.weights,
            self.meals
        );
        if !self.duplicate_refs.is_empty() {
            s.push_str(&format!(
                "\n\nset ids listed twice by the same session ({}), collapsed to one row each:",
                self.duplicate_refs.len()
            ));
            for e in &self.duplicate_refs {
                s.push_str(&format!("\n  {e}"));
            }
        }
        if !self.superset_no_reps.is_empty() {
            s.push_str(&format!(
                "\n\nsuperset notes with no rep count ({}), left as notes:",
                self.superset_no_reps.len()
            ));
            for e in &self.superset_no_reps {
                s.push_str(&format!("\n  {e}"));
            }
        }
        if !self.superset_self.is_empty() {
            s.push_str(&format!(
                "\n\nsuperset notes whose partner is the same exercise ({}), left as notes:",
                self.superset_self.len()
            ));
            for e in &self.superset_self {
                s.push_str(&format!("\n  {e}"));
            }
        }
        if !self.superset_unmapped.is_empty() {
            s.push_str(&format!(
                "\n\nsuperset notes whose partner is not in the mapping ({}), left as notes:",
                self.superset_unmapped.len()
            ));
            for e in &self.superset_unmapped {
                s.push_str(&format!("\n  {e}"));
            }
        }
        s
    }
}

fn read_file<T: DeserializeOwned + Default>(p: &Path) -> Res<BTreeMap<i64, T>> {
    if !p.exists() {
        return Ok(Default::default());
    }
    let s = fs::read_to_string(p)
        .map_err(|e| format!("Failed to read '{}' because '{e}'", p.display()))?;
    let e: Entries<T> = toml::from_str(&s)
        .map_err(|e| format!("Failed to parse '{}' because '{e}'", p.display()))?;
    Ok(e.e)
}

fn read_dir<T: DeserializeOwned + Default>(d: &Path) -> Res<BTreeMap<i64, T>> {
    let mut out = BTreeMap::new();
    if !d.exists() {
        return Ok(out);
    }
    let mut files: Vec<PathBuf> = fs::read_dir(d)
        .map_err(|e| format!("Failed to read '{}' because '{e}'", d.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().map(|x| x == "toml").unwrap_or(false))
        .collect();
    files.sort();
    for f in files {
        for (id, v) in read_file::<T>(&f)? {
            if out.insert(id, v).is_some() {
                Err(format!(
                    "id '{id}' appears twice in '{}'; refusing to import an ambiguous archive.",
                    d.display()
                ))?;
            }
        }
    }
    Ok(out)
}

fn map_of(p: &Path) -> Res<BTreeMap<String, String>> {
    if !p.exists() {
        Err(format!(
            "The mapping file '{}' does not exist.",
            p.display()
        ))?;
    }
    let s = fs::read_to_string(p)
        .map_err(|e| format!("Failed to read '{}' because '{e}'", p.display()))?;
    let m: MapFile = toml::from_str(&s)
        .map_err(|e| format!("Failed to parse '{}' because '{e}'", p.display()))?;
    Ok(m.map)
}

fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

struct Partner {
    name: String,
    rep: f64,
}

/// The partner's name and reps as written in the note, before any resolution:
/// `None` when the note names no reps at all.
fn parse_partner(note: &str) -> Option<(String, f64)> {
    let r = Regex::new(r"(?i)^su\w*\s+set\s+(.+?)\s+(\d+(?:\.\d+)?)(\s|$)").ok()?;
    let c = r.captures(note)?;
    Some((
        norm(c.get(1)?.as_str()),
        c.get(2)?.as_str().parse::<f64>().ok()?,
    ))
}

/// Resolve a note's partner name through the mapping, longest key first.
fn resolve(rest: &str, map: &BTreeMap<String, String>) -> Option<Partner> {
    let hit = map
        .iter()
        .map(|(k, v)| (norm(k), v))
        .filter(|(k, _)| !k.is_empty() && (rest == *k || rest.starts_with(&format!("{k} "))))
        .max_by_key(|(k, _)| k.len());
    hit.map(|(_, v)| Partner {
        name: v.to_owned(),
        rep: 0.0,
    })
}

struct Row {
    id: Option<i64>,
    exercise: i64,
    date: String,
    load: f64,
    rep: f64,
    note: String,
    inferred: bool,
    group: Option<usize>,
}

pub async fn import(db: &Db, dir: &Path, map_path: Option<&Path>) -> Res<ImportReport> {
    let mut r = ImportReport::default();

    let muscle_groups = read_file::<TMuscleGroup>(&dir.join("muscle_group.toml"))?;
    let places = read_file::<TPlace>(&dir.join("place.toml"))?;
    let exercises = read_file::<TExercise>(&dir.join("exercise.toml"))?;
    let foods = read_file::<TFood>(&dir.join("food.toml"))?;
    let sessions = read_dir::<TSession>(&dir.join("session"))?;
    let sets = read_dir::<TSet>(&dir.join("set"))?;
    let weights = read_dir::<TWeight>(&dir.join("weight"))?;
    let meals = read_dir::<TMeal>(&dir.join("meal"))?;
    let map = match map_path {
        Some(p) => map_of(p)?,
        None => BTreeMap::new(),
    };

    let mut referenced: BTreeSet<i64> = BTreeSet::new();
    let mut owner: HashMap<i64, i64> = HashMap::new();
    let mut duplicates: Vec<String> = Vec::new();
    for (sid, s) in &sessions {
        for id in &s.sets {
            if !sets.contains_key(id) {
                Err(format!(
                    "session '{sid}' lists set '{id}', which is not in set/."
                ))?;
            }
            match owner.insert(*id, *sid) {
                None => {}
                Some(prev) if prev == *sid => {
                    duplicates.push(format!("{id} in session {sid}"));
                }
                Some(prev) => {
                    Err(format!(
                        "set '{id}' is listed by session '{prev}' and by session '{sid}'; refusing to guess which one owns it."
                    ))?
                }
            }
            referenced.insert(*id);
        }
        if !places.contains_key(&s.place) {
            Err(format!(
                "session '{sid}' uses place '{}', which is not in place.toml.",
                s.place
            ))?;
        }
    }
    let orphans: Vec<String> = sets
        .keys()
        .filter(|id| !referenced.contains(id))
        .map(|id| id.to_string())
        .collect();
    if !orphans.is_empty() {
        Err(format!(
            "{} set(s) are not listed by any session: {}. Refusing to attach them to a guess.",
            orphans.len(),
            orphans.join(", ")
        ))?;
    }
    for (id, s) in &sets {
        if !exercises.contains_key(&s.exercise) {
            Err(format!(
                "set '{id}' uses exercise '{}', which is not in exercise.toml.",
                s.exercise
            ))?;
        }
    }
    for (id, e) in &exercises {
        for l in &e.muscle_groups {
            if !muscle_groups.contains_key(&l.group) {
                Err(format!(
                    "exercise '{id}' links muscle group '{}', which is not in muscle_group.toml.",
                    l.group
                ))?;
            }
        }
    }
    for (id, m) in &meals {
        if !foods.contains_key(&m.food) {
            Err(format!(
                "meal '{id}' uses food '{}', which is not in food.toml.",
                m.food
            ))?;
        }
    }

    let by_name: HashMap<String, i64> = exercises
        .iter()
        .map(|(id, e)| (norm(&e.name), *id))
        .collect();

    r.duplicate_refs = duplicates;

    let mut next_id = sets.keys().max().copied().unwrap_or(0);

    let mut tx = db.pool.begin().await?;

    for (id, m) in &muscle_groups {
        sqlx::query!(
            "INSERT INTO muscle_group (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            m.name,
            m.desc
        )
        .execute(&mut *tx)
        .await?;
        r.muscle_groups += 1;
    }
    for (id, p) in &places {
        sqlx::query!(
            "INSERT INTO place (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            p.name,
            p.desc
        )
        .execute(&mut *tx)
        .await?;
        r.places += 1;
    }
    for (id, e) in &exercises {
        sqlx::query!(
            "INSERT INTO exercise (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            e.name,
            e.desc
        )
        .execute(&mut *tx)
        .await?;
        r.exercises += 1;
        for l in &e.muscle_groups {
            sqlx::query!(
                "INSERT INTO exercise_muscle_group (exercise, muscle_group, amount) VALUES (?1, ?2, ?3)",
                id,
                l.group,
                l.amount
            )
            .execute(&mut *tx)
            .await?;
            r.exercise_muscle_groups += 1;
        }
    }
    for (id, f) in &foods {
        sqlx::query!(
            "INSERT INTO food (id, name, calories, protein, fat, carbohydrate, note) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            id,
            f.name,
            f.calories,
            f.protein,
            f.fat,
            f.carbohydrate,
            f.desc
        )
        .execute(&mut *tx)
        .await?;
        r.foods += 1;
    }

    for (sid, s) in &sessions {
        sqlx::query!(
            "INSERT INTO session (id, date, place, note) VALUES (?1, ?2, ?3, ?4)",
            sid,
            s.date,
            s.place,
            s.desc
        )
        .execute(&mut *tx)
        .await?;
        r.sessions += 1;

        let mut rows: Vec<Row> = Vec::new();
        let mut seen: BTreeSet<i64> = BTreeSet::new();
        for id in &s.sets {
            if !seen.insert(*id) {
                continue;
            }
            let set = &sets[id];
            let p = match parse_partner(&set.desc) {
                Some((rest, rep)) => {
                    let resolved = resolve(&rest, &map).map(|p| Partner { name: p.name, rep });
                    match resolved
                        .map(|p| (p.name.clone(), p))
                        .and_then(|(n, p)| by_name.get(&norm(&n)).copied().map(|id| (id, p)))
                    {
                        None => {
                            r.superset_unmapped
                                .push(format!("{id}: '{}' -> '{}'", set.desc, rest));
                            None
                        }
                        Some((t, _)) if t == set.exercise => {
                            r.superset_self.push(format!("{id}: '{}'", set.desc));
                            None
                        }
                        Some((_, p)) => Some(p),
                    }
                }
                None => {
                    if Regex::new(r"(?i)^su\w*\s+set\s")
                        .map(|x| x.is_match(&set.desc))
                        .unwrap_or(false)
                    {
                        r.superset_no_reps.push(format!("{id}: '{}'", set.desc));
                    }
                    None
                }
            };
            match p {
                None => rows.push(Row {
                    id: Some(*id),
                    exercise: set.exercise,
                    date: set.date.to_owned(),
                    load: set.load,
                    rep: set.rep,
                    note: set.desc.to_owned(),
                    inferred: false,
                    group: None,
                }),
                Some(p) => {
                    let group = rows.len();
                    rows.push(Row {
                        id: Some(*id),
                        exercise: set.exercise,
                        date: set.date.to_owned(),
                        load: set.load,
                        rep: set.rep,
                        note: set.desc.to_owned(),
                        inferred: false,
                        group: Some(group),
                    });
                    let pid = by_name
                        .get(&norm(&p.name))
                        .copied()
                        .ok_or(format!("'{}' vanished mid-import", p.name))?;
                    rows.push(Row {
                        id: None,
                        exercise: pid,
                        date: set.date.to_owned(),
                        load: set.load,
                        rep: p.rep,
                        note: format!("inferred from \"{}\"", set.desc),
                        inferred: true,
                        group: Some(group),
                    });
                }
            }
        }

        let mut groups: HashMap<usize, i64> = HashMap::new();
        for g in rows.iter().filter_map(|row| row.group) {
            if groups.contains_key(&g) {
                continue;
            }
            let id = sqlx::query_scalar!(
                r#"INSERT INTO superset (session) VALUES (?1) RETURNING id AS "id!""#,
                sid
            )
            .fetch_one(&mut *tx)
            .await?;
            groups.insert(g, id);
            r.supersets += 1;
        }
        for (position, row) in rows.iter().enumerate() {
            let superset = row.group.and_then(|g| groups.get(&g).copied());
            let id = match row.id {
                Some(id) => id,
                None => {
                    next_id += 1;
                    next_id
                }
            };
            sqlx::query!(
                "INSERT INTO training_set (id, session, exercise, position, date, load, rep, note, superset, inferred) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                id,
                sid,
                row.exercise,
                position as i64,
                row.date,
                row.load,
                row.rep,
                row.note,
                superset,
                row.inferred
            )
            .execute(&mut *tx)
            .await?;
            r.sets += 1;
            if row.inferred {
                r.inferred += 1;
            }
        }
    }

    for (id, w) in &weights {
        sqlx::query!(
            "INSERT INTO weight (id, date, kg, bodyfat, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            w.date,
            w.kg,
            w.bodyfat,
            w.desc
        )
        .execute(&mut *tx)
        .await?;
        r.weights += 1;
    }
    for (id, m) in &meals {
        sqlx::query!(
            "INSERT INTO meal (id, date, food, amount, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            m.date,
            m.food,
            m.amount,
            m.desc
        )
        .execute(&mut *tx)
        .await?;
        r.meals += 1;
    }

    tx.commit().await?;
    Ok(r)
}

/// Refuse an import into a database that already holds log rows. The seeded
/// `setting` and `effort` tables are allowed, because `gym db init` writes them.
#[derive(Debug, Default)]
pub struct Counts {
    pub muscle_group: i64,
    pub place: i64,
    pub exercise: i64,
    pub food: i64,
    pub session: i64,
    pub training_set: i64,
    pub weight: i64,
    pub meal: i64,
    pub superset: i64,
}

impl Counts {
    pub fn total(&self) -> i64 {
        self.muscle_group
            + self.place
            + self.exercise
            + self.food
            + self.session
            + self.training_set
            + self.weight
            + self.meal
            + self.superset
    }

    pub fn of(r: &ImportReport) -> Self {
        Self {
            muscle_group: r.muscle_groups as i64,
            place: r.places as i64,
            exercise: r.exercises as i64,
            food: r.foods as i64,
            session: r.sessions as i64,
            training_set: r.sets as i64,
            weight: r.weights as i64,
            meal: r.meals as i64,
            superset: r.supersets as i64,
        }
    }
}

pub async fn counts(db: &Db) -> Res<Counts> {
    Ok(Counts {
        muscle_group: sqlx::query_scalar!("SELECT count(*) FROM muscle_group")
            .fetch_one(&db.pool)
            .await?,
        place: sqlx::query_scalar!("SELECT count(*) FROM place")
            .fetch_one(&db.pool)
            .await?,
        exercise: sqlx::query_scalar!("SELECT count(*) FROM exercise")
            .fetch_one(&db.pool)
            .await?,
        food: sqlx::query_scalar!("SELECT count(*) FROM food")
            .fetch_one(&db.pool)
            .await?,
        session: sqlx::query_scalar!("SELECT count(*) FROM session")
            .fetch_one(&db.pool)
            .await?,
        training_set: sqlx::query_scalar!("SELECT count(*) FROM training_set")
            .fetch_one(&db.pool)
            .await?,
        weight: sqlx::query_scalar!("SELECT count(*) FROM weight")
            .fetch_one(&db.pool)
            .await?,
        meal: sqlx::query_scalar!("SELECT count(*) FROM meal")
            .fetch_one(&db.pool)
            .await?,
        superset: sqlx::query_scalar!("SELECT count(*) FROM superset")
            .fetch_one(&db.pool)
            .await?,
    })
}
