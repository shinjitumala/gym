use crate::{com::*, db::Db};
use regex::Regex;
use serde::{de::DeserializeOwned, Deserialize};
use sqlx::SqliteConnection;
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
        s.push_str(&listed(
            "set ids listed twice by the same session, collapsed to one row each",
            &self.duplicate_refs,
        ));
        s.push_str(&listed(
            "superset notes with no rep count, left as notes",
            &self.superset_no_reps,
        ));
        s.push_str(&listed(
            "superset notes whose partner is the same exercise, left as notes",
            &self.superset_self,
        ));
        s.push_str(&listed(
            "superset notes whose partner is not in the mapping, left as notes",
            &self.superset_unmapped,
        ));
        s
    }
}

fn listed(title: &str, rows: &[String]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut s = format!("\n\n{title} ({}):", rows.len());
    for e in rows {
        s.push_str(&format!("\n  {e}"));
    }
    s
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

impl Row {
    fn working(set: &TSet, id: i64) -> Self {
        Self {
            id: Some(id),
            exercise: set.exercise,
            date: set.date.clone(),
            load: set.load,
            rep: set.rep,
            note: set.desc.clone(),
            inferred: false,
            group: None,
        }
    }

    fn inferred(set: &TSet, exercise: i64, rep: f64, group: usize) -> Self {
        Self {
            id: None,
            exercise,
            date: set.date.clone(),
            load: set.load,
            rep,
            note: format!("inferred from \"{}\"", set.desc),
            inferred: true,
            group: Some(group),
        }
    }
}

struct Archive {
    muscle_groups: BTreeMap<i64, TMuscleGroup>,
    places: BTreeMap<i64, TPlace>,
    exercises: BTreeMap<i64, TExercise>,
    foods: BTreeMap<i64, TFood>,
    sessions: BTreeMap<i64, TSession>,
    sets: BTreeMap<i64, TSet>,
    weights: BTreeMap<i64, TWeight>,
    meals: BTreeMap<i64, TMeal>,
    map: BTreeMap<String, String>,
}

fn read_archive(dir: &Path, map_path: Option<&Path>) -> Res<Archive> {
    Ok(Archive {
        muscle_groups: read_file(&dir.join("muscle_group.toml"))?,
        places: read_file(&dir.join("place.toml"))?,
        exercises: read_file(&dir.join("exercise.toml"))?,
        foods: read_file(&dir.join("food.toml"))?,
        sessions: read_dir(&dir.join("session"))?,
        sets: read_dir(&dir.join("set"))?,
        weights: read_dir(&dir.join("weight"))?,
        meals: read_dir(&dir.join("meal"))?,
        map: read_map(map_path)?,
    })
}

fn read_map(map_path: Option<&Path>) -> Res<BTreeMap<String, String>> {
    match map_path {
        Some(p) => map_of(p),
        None => Ok(BTreeMap::new()),
    }
}

pub async fn import(db: &Db, dir: &Path, map_path: Option<&Path>) -> Res<ImportReport> {
    let a = read_archive(dir, map_path)?;
    let mut r = ImportReport {
        duplicate_refs: validate(&a)?,
        ..Default::default()
    };
    let mut tx = db.pool.begin().await?;
    write_reference_tables(&mut tx, &a, &mut r).await?;
    write_sessions(&mut tx, &a, &mut r).await?;
    write_weights(&mut tx, &a, &mut r).await?;
    write_meals(&mut tx, &a, &mut r).await?;
    tx.commit().await?;
    Ok(r)
}

fn validate(a: &Archive) -> Res<Vec<String>> {
    let (referenced, duplicates) = referenced_sets(a)?;
    refuse_orphans(a, &referenced)?;
    refuse_unknown_exercises(a)?;
    refuse_unknown_groups(a)?;
    refuse_unknown_foods(a)?;
    Ok(duplicates)
}

fn referenced_sets(a: &Archive) -> Res<(BTreeSet<i64>, Vec<String>)> {
    let mut referenced = BTreeSet::new();
    let mut owner = HashMap::new();
    let mut duplicates = Vec::new();
    for (sid, s) in &a.sessions {
        if !a.places.contains_key(&s.place) {
            Err(format!(
                "session '{sid}' uses place '{}', which is not in place.toml.",
                s.place
            ))?;
        }
        for id in &s.sets {
            if !a.sets.contains_key(id) {
                Err(format!(
                    "session '{sid}' lists set '{id}', which is not in set/."
                ))?;
            }
            claim(&mut owner, &mut duplicates, *sid, *id)?;
            referenced.insert(*id);
        }
    }
    Ok((referenced, duplicates))
}

fn claim(
    owner: &mut HashMap<i64, i64>,
    duplicates: &mut Vec<String>,
    sid: i64,
    id: i64,
) -> Res<()> {
    match owner.insert(id, sid) {
        None => {}
        Some(prev) if prev == sid => duplicates.push(format!("{id} in session {sid}")),
        Some(prev) => {
            Err(format!(
                "set '{id}' is listed by session '{prev}' and by session '{sid}'; refusing to guess which one owns it."
            ))?
        }
    }
    Ok(())
}

fn refuse_orphans(a: &Archive, referenced: &BTreeSet<i64>) -> Res<()> {
    let orphans: Vec<String> = a
        .sets
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
    Ok(())
}

fn refuse_unknown_exercises(a: &Archive) -> Res<()> {
    for (id, s) in &a.sets {
        if !a.exercises.contains_key(&s.exercise) {
            Err(format!(
                "set '{id}' uses exercise '{}', which is not in exercise.toml.",
                s.exercise
            ))?;
        }
    }
    Ok(())
}

fn refuse_unknown_groups(a: &Archive) -> Res<()> {
    for (id, e) in &a.exercises {
        for l in &e.muscle_groups {
            if !a.muscle_groups.contains_key(&l.group) {
                Err(format!(
                    "exercise '{id}' links muscle group '{}', which is not in muscle_group.toml.",
                    l.group
                ))?;
            }
        }
    }
    Ok(())
}

fn refuse_unknown_foods(a: &Archive) -> Res<()> {
    for (id, m) in &a.meals {
        if !a.foods.contains_key(&m.food) {
            Err(format!(
                "meal '{id}' uses food '{}', which is not in food.toml.",
                m.food
            ))?;
        }
    }
    Ok(())
}

async fn write_reference_tables(
    conn: &mut SqliteConnection,
    a: &Archive,
    r: &mut ImportReport,
) -> Res<()> {
    for (id, m) in &a.muscle_groups {
        sqlx::query!(
            "INSERT INTO muscle_group (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            m.name,
            m.desc
        )
        .execute(&mut *conn)
        .await?;
        r.muscle_groups += 1;
    }
    for (id, p) in &a.places {
        sqlx::query!(
            "INSERT INTO place (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            p.name,
            p.desc
        )
        .execute(&mut *conn)
        .await?;
        r.places += 1;
    }
    for (id, e) in &a.exercises {
        sqlx::query!(
            "INSERT INTO exercise (id, name, note) VALUES (?1, ?2, ?3)",
            id,
            e.name,
            e.desc
        )
        .execute(&mut *conn)
        .await?;
        r.exercises += 1;
        for l in &e.muscle_groups {
            sqlx::query!(
                "INSERT INTO exercise_muscle_group (exercise, muscle_group, amount) VALUES (?1, ?2, ?3)",
                id,
                l.group,
                l.amount
            )
            .execute(&mut *conn)
            .await?;
            r.exercise_muscle_groups += 1;
        }
    }
    for (id, f) in &a.foods {
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
        .execute(&mut *conn)
        .await?;
        r.foods += 1;
    }
    Ok(())
}

async fn write_sessions(conn: &mut SqliteConnection, a: &Archive, r: &mut ImportReport) -> Res<()> {
    let names: HashMap<String, i64> = a
        .exercises
        .iter()
        .map(|(id, e)| (norm(&e.name), *id))
        .collect();
    let mut next_id = a.sets.keys().max().copied().unwrap_or(0);
    for (sid, s) in &a.sessions {
        insert_session(conn, *sid, s, r).await?;
        let rows = session_rows(a, &names, s, r)?;
        write_session_rows(conn, *sid, &rows, &mut next_id, r).await?;
    }
    Ok(())
}

async fn insert_session(
    conn: &mut SqliteConnection,
    sid: i64,
    s: &TSession,
    r: &mut ImportReport,
) -> Res<()> {
    sqlx::query!(
        "INSERT INTO session (id, date, place, note, ended) VALUES (?1, ?2, ?3, ?4, ?2)",
        sid,
        s.date,
        s.place,
        s.desc
    )
    .execute(&mut *conn)
    .await?;
    r.sessions += 1;
    Ok(())
}

fn session_rows(
    a: &Archive,
    names: &HashMap<String, i64>,
    s: &TSession,
    r: &mut ImportReport,
) -> Res<Vec<Row>> {
    let mut rows = Vec::new();
    let mut seen = BTreeSet::new();
    for id in &s.sets {
        if !seen.insert(*id) {
            continue;
        }
        let set = &a.sets[id];
        let Some((p, pid)) = partner(a, names, *id, set, r) else {
            rows.push(Row::working(set, *id));
            continue;
        };
        let group = rows.len();
        rows.push(Row {
            group: Some(group),
            ..Row::working(set, *id)
        });
        rows.push(Row::inferred(set, pid, p.rep, group));
    }
    Ok(rows)
}

fn partner(
    a: &Archive,
    names: &HashMap<String, i64>,
    id: i64,
    set: &TSet,
    r: &mut ImportReport,
) -> Option<(Partner, i64)> {
    let Some((rest, rep)) = parse_partner(&set.desc) else {
        if names_a_superset(&set.desc) {
            r.superset_no_reps.push(format!("{id}: '{}'", set.desc));
        }
        return None;
    };
    let hit = resolve(&rest, &a.map).and_then(|p| {
        let name = norm(&p.name);
        names.get(&name).copied().map(|pid| (p, pid))
    });
    let Some((p, pid)) = hit else {
        r.superset_unmapped
            .push(format!("{id}: '{}' -> '{}'", set.desc, rest));
        return None;
    };
    if pid == set.exercise {
        r.superset_self.push(format!("{id}: '{}'", set.desc));
        return None;
    }
    Some((Partner { rep, ..p }, pid))
}

fn names_a_superset(note: &str) -> bool {
    Regex::new(r"(?i)^su\w*\s+set\s")
        .map(|x| x.is_match(note))
        .unwrap_or(false)
}

async fn write_session_rows(
    conn: &mut SqliteConnection,
    sid: i64,
    rows: &[Row],
    next_id: &mut i64,
    r: &mut ImportReport,
) -> Res<()> {
    let groups = write_supersets(conn, sid, rows, r).await?;
    for (position, row) in rows.iter().enumerate() {
        let superset = row.group.and_then(|g| groups.get(&g).copied());
        write_set(conn, sid, position, row, superset, next_id, r).await?;
    }
    Ok(())
}

async fn write_supersets(
    conn: &mut SqliteConnection,
    sid: i64,
    rows: &[Row],
    r: &mut ImportReport,
) -> Res<HashMap<usize, i64>> {
    let mut groups = HashMap::new();
    for g in rows.iter().filter_map(|row| row.group) {
        if groups.contains_key(&g) {
            continue;
        }
        let id = sqlx::query_scalar!(
            r#"INSERT INTO superset (session) VALUES (?1) RETURNING id AS "id!""#,
            sid
        )
        .fetch_one(&mut *conn)
        .await?;
        groups.insert(g, id);
        r.supersets += 1;
    }
    Ok(groups)
}

async fn write_set(
    conn: &mut SqliteConnection,
    sid: i64,
    position: usize,
    row: &Row,
    superset: Option<i64>,
    next_id: &mut i64,
    r: &mut ImportReport,
) -> Res<()> {
    let id = match row.id {
        Some(id) => id,
        None => {
            *next_id += 1;
            *next_id
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
    .execute(&mut *conn)
    .await?;
    r.sets += 1;
    if row.inferred {
        r.inferred += 1;
    }
    Ok(())
}

async fn write_weights(conn: &mut SqliteConnection, a: &Archive, r: &mut ImportReport) -> Res<()> {
    for (id, w) in &a.weights {
        sqlx::query!(
            "INSERT INTO weight (id, date, kg, bodyfat, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            w.date,
            w.kg,
            w.bodyfat,
            w.desc
        )
        .execute(&mut *conn)
        .await?;
        r.weights += 1;
    }
    Ok(())
}

async fn write_meals(conn: &mut SqliteConnection, a: &Archive, r: &mut ImportReport) -> Res<()> {
    for (id, m) in &a.meals {
        sqlx::query!(
            "INSERT INTO meal (id, date, food, amount, note) VALUES (?1, ?2, ?3, ?4, ?5)",
            id,
            m.date,
            m.food,
            m.amount,
            m.desc
        )
        .execute(&mut *conn)
        .await?;
        r.meals += 1;
    }
    Ok(())
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
        muscle_group: count(db, "SELECT count(*) FROM muscle_group").await?,
        place: count(db, "SELECT count(*) FROM place").await?,
        exercise: count(db, "SELECT count(*) FROM exercise").await?,
        food: count(db, "SELECT count(*) FROM food").await?,
        session: count(db, "SELECT count(*) FROM session").await?,
        training_set: count(db, "SELECT count(*) FROM training_set").await?,
        weight: count(db, "SELECT count(*) FROM weight").await?,
        meal: count(db, "SELECT count(*) FROM meal").await?,
        superset: count(db, "SELECT count(*) FROM superset").await?,
    })
}

async fn count(db: &Db, sql: &'static str) -> Res<i64> {
    Ok(sqlx::query_scalar(sql).fetch_one(&db.pool).await?)
}
