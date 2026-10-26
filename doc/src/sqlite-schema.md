# The SQLite schema

One database file holds everything. The schema below is
`migrations/0001_init.sql` in the repository, character for character; the
migration from the TOML archive is described after it.

## Conventions

- **The schema is a migration file.** The SQL below is `migrations/0001_init.sql`,
  applied by `sqlx::migrate!` at startup and against a scratch database when the
  commit-time query cache is refreshed. There is no second copy of the schema to
  fall out of step.
- **Every query is a macro.** `sqlx::query!`/`query_as!`/`query_scalar!` check
  each statement against that migration at build time. A column rename that a
  query still uses is a compile error, which is the point. Statements are
  literals: a screen with an optional filter binds parameters, it does not
  build SQL — `WHERE (?1 IS NULL OR date >= ?1) AND (?2 IS NULL OR date <= ?2)`.
- **Connection setup is per connection, not per database.** WAL, foreign keys
  and the busy timeout are set through `SqliteConnectOptions`
  (`.journal_mode(Wal)`, `.foreign_keys(true)`,
  `.busy_timeout(Duration::from_secs(5))`), because SQLite forgets two of the
  three whenever a connection is opened.
- **Dates are RFC3339 `TEXT`.** The existing log already uses them, they sort
  chronologically as text in the same offset, and their precision varies (some
  rows carry nanoseconds), which a SQL type would have to round.
- **Ids are `INTEGER PRIMARY KEY`.** The importer preserves the old ids, so a
  row's number survives the migration and the old notes stay readable.
- **Free text is `note`.** `desc` is a SQL keyword and a column cannot be named
  it without quoting; `note` is also the word the prompts use.
- **Unknown is `NULL`, measured zero is `0`.** Body fat, and a food's protein,
  fat and carbohydrate, are nullable so "not known" is not the same row as
  "none".

## Tables

The migration, in full:

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE muscle_group (
    id     INTEGER PRIMARY KEY,
    name   TEXT NOT NULL UNIQUE,
    note   TEXT NOT NULL DEFAULT ''
);

CREATE TABLE exercise (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    note       TEXT NOT NULL DEFAULT '',
    video_url  TEXT
);

CREATE TABLE exercise_muscle_group (
    exercise      INTEGER NOT NULL REFERENCES exercise(id) ON DELETE CASCADE,
    muscle_group  INTEGER NOT NULL REFERENCES muscle_group(id) ON DELETE RESTRICT,
    amount        REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (exercise, muscle_group)
);

CREATE TABLE place (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    note  TEXT NOT NULL DEFAULT ''
);

CREATE TABLE phase (
    id             INTEGER PRIMARY KEY,
    kind           TEXT NOT NULL CHECK (kind IN ('bulk', 'cut')),
    start          TEXT NOT NULL,
    target_weight  REAL,
    target_rate    REAL NOT NULL DEFAULT 1.0,
    target_rep     INTEGER NOT NULL DEFAULT 8,
    note           TEXT NOT NULL DEFAULT ''
);

CREATE TABLE session (
    id      INTEGER PRIMARY KEY,
    date    TEXT NOT NULL,
    place   INTEGER NOT NULL REFERENCES place(id) ON DELETE RESTRICT,
    phase   INTEGER REFERENCES phase(id) ON DELETE SET NULL,
    deload  INTEGER NOT NULL DEFAULT 0 CHECK (deload IN (0, 1)),
    note    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE superset (
    id       INTEGER PRIMARY KEY,
    session  INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE
);

CREATE TABLE effort (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE,
    sort  INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

CREATE TABLE training_set (
    id        INTEGER PRIMARY KEY,
    session   INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    exercise  INTEGER NOT NULL REFERENCES exercise(id) ON DELETE RESTRICT,
    position  INTEGER NOT NULL,
    date      TEXT NOT NULL,
    load      REAL NOT NULL,
    rep       REAL NOT NULL,
    note      TEXT NOT NULL DEFAULT '',
    effort    INTEGER REFERENCES effort(id) ON DELETE SET NULL,
    superset  INTEGER REFERENCES superset(id) ON DELETE SET NULL,
    inferred  INTEGER NOT NULL DEFAULT 0 CHECK (inferred IN (0, 1)),
    UNIQUE (session, position)
);

CREATE TABLE exercise_list (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE
);

CREATE TABLE exercise_list_item (
    list      INTEGER NOT NULL REFERENCES exercise_list(id) ON DELETE CASCADE,
    exercise  INTEGER NOT NULL REFERENCES exercise(id) ON DELETE RESTRICT,
    position  INTEGER NOT NULL,
    PRIMARY KEY (list, exercise),
    UNIQUE (list, position)
);

CREATE TABLE session_list_item (
    session   INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE,
    exercise  INTEGER NOT NULL REFERENCES exercise(id) ON DELETE RESTRICT,
    position  INTEGER NOT NULL,
    PRIMARY KEY (session, exercise),
    UNIQUE (session, position)
);

CREATE TABLE weight (
    id       INTEGER PRIMARY KEY,
    date     TEXT NOT NULL,
    kg       REAL NOT NULL,
    bodyfat  REAL,
    note     TEXT NOT NULL DEFAULT ''
);

CREATE TABLE food (
    id            INTEGER PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    calories      REAL NOT NULL,
    protein       REAL,
    fat           REAL,
    carbohydrate  REAL,
    note          TEXT NOT NULL DEFAULT ''
);

CREATE TABLE meal (
    id      INTEGER PRIMARY KEY,
    date    TEXT NOT NULL,
    food    INTEGER NOT NULL REFERENCES food(id) ON DELETE RESTRICT,
    amount  REAL NOT NULL DEFAULT 1.0,
    note    TEXT NOT NULL DEFAULT ''
);

CREATE TABLE meal_preset (
    id         INTEGER PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    note       TEXT NOT NULL DEFAULT '',
    last_used  TEXT
);

CREATE TABLE meal_preset_item (
    preset  INTEGER NOT NULL REFERENCES meal_preset(id) ON DELETE CASCADE,
    food    INTEGER NOT NULL REFERENCES food(id) ON DELETE RESTRICT,
    amount  REAL NOT NULL DEFAULT 1.0,
    PRIMARY KEY (preset, food)
);

CREATE INDEX session_date ON session (date);
CREATE INDEX session_place_date ON session (place, date);
CREATE INDEX training_set_exercise_date ON training_set (exercise, date);
CREATE INDEX meal_date ON meal (date);
CREATE INDEX weight_date ON weight (date);
```

## What the tables mean

- **`training_set.session` replaces the session's `sets` list.** Membership is a
  column now, so a set cannot be orphaned by a hand edit. `position` is the
  order within the session and is unique there; that is the order the set was
  entered.
- **`superset` is a group.** A `NULL` `training_set.superset` is a standalone
  set. Members are ordered by their own `position`, and because a superset's
  members share a load by definition, only reps differ between them. The web UI
  no longer records a group — a round of exercises is the session list's loop —
  so this column is history: it is read and shown on the edit page, and nothing
  writes it.
- **`inferred` marks a reconstructed row**, one the importer built from a note
  rather than a recorded set. The analysis counts it; a human can correct it.
- **`session.phase` is recorded, not derived.** Starting a new phase does not
  reinterpret old sessions.
- **`session_list_item` is a snapshot.** Starting a session from a list copies
  the list into the session, so editing the list later does not rewrite history
  and ticking items off is a property of that session.
- **`meal_preset.last_used` is an ordering key**, written when the preset is
  applied. The preset list is ordered by it, so the common breakfast comes
  first without a separate frequency table.
- **Nothing about a PR or an estimated 1RM is stored.** Both are computed from
  `training_set`, so correcting a set re-derives the badge.
- **`setting` holds the numbers that are policy, not data.** They are stored so
  they can be changed without a rebuild, and so a past analysis can be explained
  by the values that were in force. Each is read through a typed accessor in the
  code, with the default applied when the row is absent:

  | Key | Default | Meaning |
  |-----|---------|---------|
  | `deload_weeks` | `6` | Weeks after the last deload before the reminder appears. |
  | `trend_window_days` | `21` | Trailing window for the weight trend and the maintenance estimate. |
  | `trend_min_weighins` | `14` | Weigh-ins needed inside the window before a slope is shown. |
  | `maintenance_min_days` | `10` | Logged days needed inside the window before maintenance is shown. |

### Constraints worth knowing

- Deleting an `exercise` or a `food` that a set or a meal still refers to is
  refused (`ON DELETE RESTRICT`). Merge is the operation for a duplicate, not a
  delete.
- Deleting a `session` deletes its sets and its list, and leaves the superset
  groups to be deleted with it (`ON DELETE CASCADE`).
- `float` columns are `REAL` — the log already carries fractional loads such as
  `28.75`.

## Migrating the TOML archive

`gym db import --dir <archive>` reads the directory the previous version wrote
and fills an empty database. It runs in one transaction: failing part-way
leaves nothing behind, so it is run into a fresh file.

The real run is:

```bash
$ gym db init
$ gym db import --dir ~/g/gym_data/d
```

with `db` in the config pointing at the new database file and `~/g/gym_data`
kept as the git archive of the TOML era — the importer only reads from it. Add
`--map <file>` to reconstruct supersets; without it every superset note stays a
note.

| Old | New | Note |
|-----|-----|------|
| `muscle_group.toml` | `muscle_group` | 16 rows. |
| `place.toml` | `place` | 6 rows. |
| `exercise.toml` | `exercise` + `exercise_muscle_group` | 73 exercises, 52 group links. The `muscle_groups` array becomes one link row per entry, with its `amount`. |
| `food.toml` | `food` | 123 rows. A stored `0.0` stays `0.0`: the old format cannot say whether it was unknown or measured. Only rows entered after the migration can be `NULL`. |
| `session/*.toml` | `session` | 368 rows. `sets` becomes `training_set.session` and `training_set.position`; the list order is the entry order. |
| `set/*.toml` | `training_set` | 9 664 rows. The `session` field is **ignored** — it is `0` for most of the archive and the session's list is the only membership record. A set no list mentions stops the import rather than being attached to a guess, and so does a set two sessions both list. |
| `weight/*.toml` | `weight` | 312 rows. `bodyfat = 0.0` is imported as `0.0`, not `NULL`, for the same reason as food. |
| `meal/*.toml` | `meal` | 7 529 rows. |
| — | `effort` | Seeded with the vocabulary the notes already use: `fail`, `form fail`, `very hard`, `hard`, `incomplete`, `deload`, `easy`, and `warmup`. The first three are the rungs the recommendation compares; the rest are recorded and read by name, and `warmup` is left out of every comparison and count. |
| — | `setting` | Seeded with the four defaults above (`deload_weeks = 6`, `trend_window_days = 21`, `trend_min_weighins = 14`, `maintenance_min_days = 10`). |
| — | `phase`, `meal_preset`, `meal_preset_item`, `exercise_list`, `exercise_list_item`, `session_list_item`, `superset` | Empty except for the supersets reconstructed below. |

Old ids are preserved, so a row's number survives the migration and the notes
that mention one stay readable. A partner row the importer invents takes the
next id above the archive's highest set id.

### What the archive gets wrong, and what the import does about it

- **Three sets are listed twice by their own session** — `1206` (session 36),
  `1552` (session 44) and `1718` (session 50) appear twice in one `sets` list.
  The import collapses the repetition to one row and lists the three in its
  report. No set is listed by two *different* sessions; that case stops the
  import.
- **The set's `session` field is wrong almost everywhere** (8 861 of 9 664 rows
  say `0`), which is why it is ignored.

### Supersets from the notes

The archive records the second half of a superset as prose in the first half's
note: `desc = "Super set side delts 18 reps"`. The logged set carries the load
and reps of the exercise that was recorded, and the partner's numbers exist
only in that sentence. 228 notes match, most of them on `cable Y raise 17H LR`
(or `Cable Y Raise Pin`), whose partner is a side-delt raise.

With a mapping file, each parseable note becomes a group:

1. Take the logged set; create a `superset` for its session.
2. Resolve the partner name in the note to one exercise. The strings seen are
   `side delts` (215), `aide delts`, `sode delts`, `sidr delts`,
   `sit side delts`, `side dets`, `super side delts`, `y raise` and `y`; the
   resolution is a row in the mapping file, because which side-delt variation
   the note means is not in the data.
3. Insert a `training_set` for that exercise with **the same date, the same
   load** (a superset shares the load; only reps differ), the reps parsed from
   the sentence, `inferred = 1`, and the position immediately after the logged
   set. Every later set in the session shifts up one, so positions stay
   contiguous and unique.
4. Put both sets in the group.
5. Leave the note as it is. It is the evidence for the row that was created,
   and rewriting it would destroy the only record of how the row came to be.

The 2026-05 archive yields 214 inferred rows and 214 groups with a mapping
covering `side delts` and the Y raises. These are reported rather than guessed,
for a human to enter by hand:

- the 3 notes with no usable rep count (`Super set Y raise`, `Super set Y`,
  `Super set side delts 16x2 reps`);
- the notes on a side-delt set itself, where the partner resolves to the same
  exercise and a group of one would be nonsense;
- the 10 notes whose partner spelling is not in the mapping (`sode delts`,
  `aide delts`, `sidr delts`, `side dets`, `side delts:`), which is the signal
  to add a row to the mapping file rather than to fix the importer.

### The exercise cleanup

73 exercises, 40 of which have no muscle group, and names that are one
hand-typed session away from each other: `Fkat Benxh`, `cable side delts 17H
LR` beside `cable side delts 3H LR`, dashes and digits that may or may not be
the same movement. Two repairs, both from the intervention tab:

- **Rename** fixes a spelling and touches nothing else.
- **Merge** takes a set of exercises that are one movement and leaves one of
  them: the others' sets and list items are re-pointed to the survivor, their
  `exercise_muscle_group` links are unioned (the survivor's `amount` wins on a
  conflict), and the merged-away rows are deleted.

Whether two names are one movement is a human judgement — the height and angle
in a cable-variation name may be the point — so every merge is a reviewed pair
in a mapping file, not a rule. The survivor's name is the one the analysis will
show, so the mapping is also where a badly named exercise gets a better name.

The mapping file is a plain table of `from` and `to` names, kept with the
migration scripts in the data directory rather than in this repository: it is a
fact about one person's log, not about the program. The importer applies it
while importing, so a re-import produces the same result.

## Verifying an import

An import is correct when these hold. The archive import of 2026-05 was checked
against all five, and `test/import.rs` and `test/store.rs` pin the ones that can
be pinned without the archive:

- **Row counts** match the table above, per table.
- **No orphans**: `PRAGMA foreign_key_check` returns nothing.
- **Every old id is present**: the ids in the new table equal the ids in the old
  files, for each of the eight imported kinds — checked for sets, meals,
  weigh-ins and sessions on the real archive, with no difference in either
  direction.
- **Membership matches**: each session's set ids equal its old `sets` list, with
  no mismatch across 368 sessions.
- **The arithmetic agrees**: `gym food`'s totals for a day, computed by the
  query the command runs and by the old binary's algorithm straight from the
  archive, are equal to the cent — 3163.84 kcal, 202.06 g protein, 112.29 g fat
  and 290.23 g carbohydrate for 2026-05-13.
- **The history block agrees**: `gym new` prints the same three sessions, in the
  same order with the same loads and reps, as the old binary on the same data.
