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

INSERT INTO effort (name, sort) VALUES
    ('fail', 0),
    ('form fail', 1),
    ('very hard', 2),
    ('hard', 3),
    ('incomplete', 4),
    ('deload', 5),
    ('easy', 6);

INSERT INTO setting (key, value) VALUES
    ('deload_weeks', '6'),
    ('trend_window_days', '21'),
    ('trend_min_weighins', '14'),
    ('maintenance_min_days', '10');
