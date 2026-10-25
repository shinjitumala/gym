# Commands

> **What this page describes.** The commands of the running build. The rebuild
keeps their prompts, adds `gym db`, turns `gym web` into the UI, and changes
`gym sync`'s transport; see [the overhaul](overhaul.md) and
[its acceptance criteria](features.md).

```text
$ gym
Expected an act.
? Choose an action.  
> weight Add weight data.
  web    Runs a local web server.
  sync   Sync with remote.
  new    New session at the gym to the buffer.
  food   Add food data.
[↑↓ to move, enter to select, type to filter]
```

Every command is a sequence of prompts, and every prompt accepts ESC: on a
list it means "none of these", on a text field it skips an optional value, on
the set and exercise loops it ends that loop. Each command appends rows to the
data directory ([the data files](data-files.md)) and prints nothing else
except where noted.

## `gym weight`

Asks for the date (defaulting to now), the weight in kilograms, the body fat
percentage and a note, then appends one row to `weight/YYYY-MM.toml`.

```text
$ gym weight
? When did you measure? 2026-04-01T22:18:45+00:00
? Weight (kg) 69.6
? Bodyfat (%) 13.7
? Note
```

## `gym new`

Logs a training session. It asks for the place, the training time and a
description, then loops over exercises until ESC:

1. **Exercise.** Types ahead against the exercises already trained at the
   chosen place. A name that is not in the list is created as a new exercise,
   with no muscle groups attached.
2. **History.** The sets of the last three sessions of that exercise at that
   place are printed, oldest data first, as `date:` followed by one `load x rep`
   row per set:

   ```text
   ? Exercise bench press
   2026-10-25T08:54:17+09:00:
   60 x 8
   ? load
   [Press ESC if done with exercise]
   ```

3. **load → rep → Notes**, repeated. ESC on `load` finishes the exercise and
   returns to the exercise prompt; ESC there finishes the session.

Each set is appended to `set/YYYY-MM-DD.toml` as soon as its notes are
entered, and the session is written to `session/YYYY-MM.toml` when the command
ends. A session that logs no sets is still written.

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
  and is then `0.0`), and a description. The new food is appended to
  `food.toml` and used for this meal.
- **Amount** is a multiplier on the food's numbers, defaulting to `1`. `2` of
  an 80 kcal unit is 160 kcal.
- Each meal is appended to `meal/YYYY-MM-DD.toml`. `Add more food?` defaults to
  `No`, which ends the command and prints the day's totals again.

## `gym sync`

Commits and publishes the data repository:

```text
$ gym sync
+ dir=/home/me/g/gym_data
+ git -C /home/me/g/gym_data pull
...
There is nothing to commit.
+ mapfile -t a
```

1. `git -C <repo> pull`.
2. If `git status --porcelain` reports changes, `git add <repo>` and
   `git commit -m upd.`; otherwise it prints `There is nothing to commit.`
3. `git push <remote> --all` for every configured remote, announcing each as
   `Pushing to '<remote>'...`.

The steps run as shell scripts with tracing on, so the `+` lines above are
normal output. A failing `git pull` prints git's message and does not stop the
command — the commit and push still run. `gym sync` needs `git` in `PATH` and a
repository whose remotes and branch tracking are already configured.

## `gym web`

Not implemented yet. The command parses an optional address (default
`0.0.0.0:8080`) and then panics:

```text
$ gym web
thread 'main' panicked at ...:
not yet implemented
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

Exit status 101. Reading the data currently means reading the TOML files
directly. In [the overhaul](overhaul.md#the-web-ui) this command becomes the
local web UI, on `127.0.0.1:8080` by default instead of `0.0.0.0`.
