# Commands

`gym --help` lists the commands, and every command answers the same question
about itself:

```text
$ gym --help
Usage: gym <act>
Acts:
weight Add weight data.
web    Runs the local web UI on loopback.
sync   Copy the database to the configured remote.
new    New session at the gym to the buffer.
food   Add food data.
db     The database.

$ gym db --help
Usage: gym db <act>
Acts:
init   Create an empty database at the configured path.
import Fill an empty database from a TOML archive directory.

$ gym web --help
Usage: gym web <opts...>
Options:
--addr Req<String> Socket address.  (default: 127.0.0.1:8080)
```

`--version` names the build, which is what to quote in a bug report:

```text
$ gym --version
gym v0.10.0: x86_64-unknown-linux-gnu, release
A local-first fitness tracker for progressive overload, body weight and food.
Repository: https://github.com/shinjitumala/gym, commit = 3c83953, dirty = false
```

Both answer without a configuration file, so they work on a machine where `gym`
has just been installed. Neither is an act: `gym` with no argument at all is a
mistake, and says so:

```text
$ gym
Expected act.
Use this command for help:
gym --help
```

A command that fails prints what went wrong and `Aborting.`, then exits with
status 1, so a script can tell a refusal from a success:

```text
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/missing.toml gym food
Failed to open file '/tmp/gym-try/missing.toml' because 'No such file or directory (os error 2)'.
Aborting.
```

Every prompt in every command accepts ESC. On a text field it skips an
optional value; on the set and exercise loops it ends that loop; on the food
list it opens food registration instead of picking. Each command writes inside
one transaction per row, and nothing runs in the background.

## `gym db init`

Creates the database at `db` with the schema, the seven seeded effort presets
and the four settings:

```text
$ gym db init
Created '/home/me/g/gym_data/gym.db'.
Seeded the effort presets and the four settings.
```

It refuses a file that is already a database and refuses a path whose directory
is missing, so a second run cannot wipe a log.

## `gym db import`

Fills an **empty** database from the TOML archive the previous version wrote:

```text
$ gym db init
$ gym db import --dir ~/g/gym_data/d
muscle groups    16
places           6
exercises        73
exercise groups  52
foods            123
sessions         368
sets             9878 (214 inferred, 214 supersets)
weigh-ins        312
meals            7529
```

- `--dir` is required: the directory holding `exercise.toml`, `place.toml`,
  `food.toml`, `muscle_group.toml` and the `set/`, `session/`, `weight/` and
  `meal/` directories.
- `--map` takes a TOML table that resolves a superset note's partner name to an
  exercise name. Without it, every superset note stays a note.

The import is all-or-nothing: it runs in one transaction and rolls back on the
first inconsistency, so a failure leaves an empty database. It refuses to run
into a database that already holds log rows, and it stops rather than guessing
when a set belongs to no session, to two sessions, or to an exercise that does
not exist. See [the schema page](sqlite-schema.md#migrating-the-toml-archive)
for what it reconstructs and what it reports instead.

## `gym weight`

Asks for the date (defaulting to now), the weight in kilograms, the body fat
percentage and a note.

```text
$ gym weight
? When did you measure? 2026-04-01T22:18:45+00:00
? Weight (kg) 69.6
? Bodyfat (%) 
[You can press ESC if unknown]
? Note
```

ESC on `Bodyfat (%)` stores `NULL`, which is different from a measured `0.0`.
Two weigh-ins in one day are both kept.

## `gym new`

Logs a training session. It asks for the place, the training time and a
description, then loops over exercises until ESC:

1. **Exercise.** Types ahead against the exercises already trained at the
   chosen place. A name that is not in the list is created as a new exercise.
2. **History.** The sets of the last three sessions of that exercise at that
   place are printed, newest session first, as `date:` followed by one
   `load x rep note` row per set:

   ```text
   ? Exercise bench press
   2026-05-12T06:54:37+09:00:
   97.5 x 6  Fail.
   125  x 1  Complete. New PR.
   ? load
   [Press ESC if done with exercise]
   ```

3. **load → rep → Notes**, repeated. ESC on `load` finishes the exercise and
   returns to the exercise prompt; ESC there finishes the session.

Each set is written as soon as its note is entered, with a position one past
the last set of that session, so the order of entry is preserved. A session
that logs no sets is still recorded.

## `gym food`

Logs meals and prints the running totals for the current local day before and
after. Totals cover local midnight to local midnight; the first line printed is
that range.

```text
$ gym food
2026-10-25T00:00:00.487863388+09:00 2026-10-26T00:00:00.487863388+09:00
Today's total:
calories kcal protein g fat g carbohydrate g
160.00        13.26     11.02 0.32
? When did you eat? 2026-10-25T08:52:53+09:00
? What did you eat? (calories kcal, protein g, fat g, carbohydrates g) boiled egg 80.0 6.6 5.5 0.2
[Press ESC to register unknown food]
? Amount (1) 2
[Multiplier]
? desc breakfast
? Add more food? (y/N) No
```

- **Picking a food.** The list shows each food with its per-unit numbers. ESC
  opens the registration prompts instead: name (must not already exist),
  calories, then protein, fat and carbohydrate (each may be skipped with ESC
  and is stored as `NULL`, not `0`), and a description.
- **Amount** is a multiplier on the food's numbers, defaulting to `1`. `2` of
  an 80 kcal unit is 160 kcal.
- Each meal is written with the timestamp of the day you answered. `Add more
  food?` defaults to `No`, which ends the command and prints the totals again.

## `gym sync`

Copies the database to `remote`, from a consistent snapshot:

```text
$ gym sync
Snapshot '/tmp/gym-sync-4242.db' (1810432 bytes).
gym-sync-4242.db
         32,768B   1%        0B/s    0:00:00
      1,810,432B 100%        1.7GB/s  0:00:00 (xfr#1, to-chk=0/1)
Copied to '/home/me/g/gym_data/gym.db'.
```

1. `VACUUM INTO` a temporary file, so the copy is consistent even while the
   command's own pool or another process is writing.
2. `rsync -uP` that file to `remote`.
3. Delete the temporary file, whether or not the transfer succeeded.

A failed transfer exits non-zero and says so. Nothing is ever pulled back: the
copy is an archive, never a second writer.

## `gym web`

Serves the gym, food and weight pages on loopback:

```text
$ gym web
gym web on http://127.0.0.1:8080
```

`--addr` moves it. A build without the embedded bundle — a plain `cargo build`,
rather than `./install.sh` or `./deploy.sh` — refuses to start and says so. See
[the web UI](web.md) for the pages and what they show, and
[the requirements](requirements.md) for the behaviour they are judged against.
