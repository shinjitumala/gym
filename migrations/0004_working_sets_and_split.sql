-- A preset says how much work each exercise is: the number of working sets it
-- is there to do. "Working" is the effort vocabulary's business, not this
-- table's: a set logged as a warmup is counted by nothing, so the target is
-- compared against the session's non-warmup sets only.
--
-- The default is what every preset written before this column meant. Three
-- working sets is the usual prescription for an accessory and the number the
-- counts were read against in the log this came from, so an existing preset
-- becomes three sets per exercise rather than a count of zero that would read
-- as "done before it started".
ALTER TABLE exercise_list_item ADD COLUMN target_sets INTEGER NOT NULL DEFAULT 3;

-- The checklist is a snapshot of the preset as it was applied, so the target is
-- copied into the session rather than read back through the preset: rewriting a
-- preset must not move the goalposts of a session already under way, and an
-- exercise added to the list on its own has no preset to take one from.
ALTER TABLE session_list_item ADD COLUMN target_sets INTEGER;

-- A split is a named week: which preset each weekday asks for. One row per
-- weekday, and `active` picks the split a starting session is offered from, so
-- a second split can be kept without being in force. A partial index is what
-- makes "at most one active" a rule of the schema rather than of the code.
--
-- Weekdays are ISO: 1 is Monday and 7 is Sunday, the numbering chrono's
-- `number_from_monday` writes and every reader of a calendar expects a week to
-- start on.
CREATE TABLE split (
    id      INTEGER PRIMARY KEY,
    name    TEXT NOT NULL UNIQUE,
    active  INTEGER NOT NULL DEFAULT 0 CHECK (active IN (0, 1))
);

CREATE UNIQUE INDEX split_one_active ON split (active) WHERE active = 1;

CREATE TABLE split_day (
    split    INTEGER NOT NULL REFERENCES split(id) ON DELETE CASCADE,
    weekday  INTEGER NOT NULL CHECK (weekday BETWEEN 1 AND 7),
    preset   INTEGER NOT NULL REFERENCES exercise_list(id) ON DELETE RESTRICT,
    PRIMARY KEY (split, weekday)
);
