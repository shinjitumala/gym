# Overview

`gym` records a training log, weigh-ins and meals in plain TOML files on the
machine that reads them. There is no server, no account and no database
engine: the data directory is the database, and it belongs to the user, who can
read it with `cat`, edit it with an editor, and back it up with `git`.

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
- **A local history that a human owns.** Everything is a TOML file inside one
  directory, which is also a git repository that `gym sync` pulls, commits and
  pushes.

## The invariants worth knowing

- **Append-only.** A command adds rows; no command edits or deletes an existing
  one. A hand edit of the TOML files is therefore safe from the tool, and the
  tool's own writes never rewrite rows it did not create in that run.
- **Directories and files must already exist.** The data directory, its
  sub-directories and its index files are validated at startup; see
  [Configuration](configuration.md) and [the data files](data-files.md) for the
  smallest directory the tool accepts.
- **Interactive only.** Every command except `gym web` prompts for input on the
  terminal. There is no batch mode.
- **ESC means "not this one".** At every prompt, ESC cancels the current
  question: on the set loop it ends the exercise, on the exercise loop it ends
  the session, on the food list it opens food registration instead of picking.

## Pages

- [Configuration](configuration.md) — the config file, where it is looked up,
  and what is validated.
- [The data files](data-files.md) — the directory layout and the schema of each
  TOML file.
- [Commands](commands.md) — every command, the questions it asks, and a worked
  example with real output.
- [Features and acceptance criteria](features.md) — the list of things this
  program is expected to do, each with the criteria that decide it is done.
