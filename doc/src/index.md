# Overview

`gym` records a training log, weigh-ins and meals in one local SQLite database.
There is no server and no account: the file is the database, and it belongs to
the user, who can copy it, back it up, and open it with `sqlite3`.

The commands are a set of prompts. Each asks a handful of questions, writes a
row, and exits; nothing runs in the background and nothing is kept in memory
between commands.

The tool is a set of prompts. Each command asks a handful of questions, appends
the answers to the data directory, and exits; nothing runs in the background
and nothing is kept in memory between commands.

## What it is for

- **Progressive overload.** When a set is logged, the last three sessions of
  that exercise at that place are printed first, so the next set can be chosen
  against what was actually lifted.
- **A food diary with today's running totals.** Calories, protein, fat and
  carbohydrate for the current local day are printed before and after logging.
- **Body weight and body fat over time.** Weigh-ins accumulate in monthly
  files, one row per day.
- **A local history that a human owns.** Everything is one file that
  `gym sync` rsyncs to a destination of the user's choosing. The TOML
  directories of the previous version remain as the archive and as what
  `gym db import` reads.

## The invariants worth knowing

- **Add-only for now.** A command adds rows; no command edits or deletes an
  existing one. Editing arrives with the intervention tabs on
  [the overhaul](overhaul.md).
- **The database file must be created once.** `gym db init` writes the schema;
  every other command refuses to run against a missing file, and refuses a path
  that is a directory. See [Configuration](configuration.md).
- **Integrity is the schema's job.** A set's session and exercise, and a meal's
  food, are foreign keys enforced by SQLite, so a row cannot point at something
  that is not there.
- **Interactive only.** Every command except `gym web` prompts for input on the
  terminal. There is no batch mode.
- **ESC means "not this one".** At every prompt, ESC cancels the current
  question: on the set loop it ends the exercise, on the exercise loop it ends
  the session, on the food list it opens food registration instead of picking.

## Pages

- [Configuration](configuration.md) — the config file, where it is looked up,
  and what is validated.
- [Commands](commands.md) — every command, the questions it asks, and a worked
  example with real output.
- [The TOML archive](data-files.md) — the previous version's file layout, kept
  as the import source.
- [Features and acceptance criteria](features.md) — the list of things this
  program is expected to do, each with the criteria that decide it is done.
- [The SQLite schema](sqlite-schema.md) — the tables, and the migration from the
  TOML archive.
- [The web UI](web.md) — `gym web`: the three modes, how a day is recorded, and
  how the site is built and tested.
- [Requirements for the web UI](requirements.md) — the scenarios the pages are
  judged against, and the recency and effort rules behind them.
- [How the code is written](code.md) — the rules the source follows: no comments,
  one step per call, one feature per module, data plus systems, and the internal
  dependency's requirement.
- [The overhaul](overhaul.md) — the rebuild onto SQLite and a local web UI: the
decisions, the pages, and the order of work.
