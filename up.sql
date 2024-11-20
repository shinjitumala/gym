CREATE TABLE exercise (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    desc TEXT NOT NULL
);

CREATE TABLE musclegroup (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    desc TEXT NOT NULL
);

INSERT INTO musclegroup (name, desc) VALUES
    ('pecs',''), 
    ('side delts',''), 
    ('front delts',''), 
    ('rear delts',''), 
    ('biceps',''), 
    ('triceps',''), 
    ('forearms',''), 
    ('abs',''), 
    ('obqliques',''), 
    ('traps',''), 
    ('lats',''), 
    ('erectors',''), 
    ('glutes',''), 
    ('quads',''), 
    ('hamstrings',''), 
    ('calves','');

CREATE TABLE exercise2musclegroup (
    id INTEGER PRIMARY KEY NOT NULL,
    exercise INTEGER NOT NULL REFERENCES exercise(id) ON DELETE CASCADE ON UPDATE RESTRICT,
    musclegroup INTEGER NOT NULL REFERENCES musclegroup(id) ON DELETE CASCADE ON UPDATE RESTRICT,
    amount REAL NOT NULL CHECK(0 < amount),
    UNIQUE(exercise, musclegroup)
);

CREATE TABLE place (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    desc TEXT NOT NULL
);

CREATE TABLE _set (
    id INTEGER PRIMARY KEY NOT NULL,
    exercise INTEGER NOT NULL REFERENCES exercise(id) ON DELETE RESTRICT ON UPDATE RESTRICT,
    load REAL NOT NULL CHECK(0 < load),
    rep REAL NOT NULL CHECK(0 < rep),
    tmax REAL NOT NULL CHECK(0 < tmax),
    desc TEXT NOT NULL
);

CREATE TABLE session (
    id INTEGER PRIMARY KEY NOT NULL,
    place INTEGER NOT NULL REFERENCES place(id) ON DELETE RESTRICT ON UPDATE RESTRICT,
    date BIGINT NOT NULL
);

CREATE TABLE session2set (
    id INTEGER PRIMARY KEY NOT NULL,
    session INTEGER NOT NULL REFERENCES session(id) ON DELETE CASCADE ON UPDATE RESTRICT,
    _set INTEGER NOT NULL REFERENCES _set(id) ON DELETE CASCADE ON UPDATE RESTRICT,
    UNIQUE(session, _set)
);

CREATE TABLE weight (
    id INTEGER PRIMARY KEY NOT NULL,
    date BIGINT NOT NULL,
    kg REAL NOT NULL,
    bodyfat REAL NOT NULL,
    desc TEXT NOT NULL
);

CREATE TABLE food (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    calories REAL NOT NULL,
    protein REAL,
    fat REAL,
    carbohydrate REAL,
    desc TEXT NOT NULL
);

CREATE TABLE meal (
    id INTEGER PRIMARY KEY NOT NULL,
    date BIGINT NOT NULL,
    food INTEGER NOT NULL REFERENCES food(id) ON DELETE CASCADE ON UPDATE RESTRICT,
    amount REAL NOT NULL,
    desc TEXT NOT NULL
);
