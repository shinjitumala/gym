CREATE TABLE exercise (
    id INTEGER PRIMARY KEY NOT NULL,
    name VARCHAR(64) UNIQUE NOT NULL,
    desc TEXT NOT NULL
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
    rep REAL NOT NULL CHECK(0 < load),
    theoretical_max REAL NOT NULL CHECK(0 < load),
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
