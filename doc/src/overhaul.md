# The overhaul

The program is being rebuilt around a SQLite database and a local web UI. The
CLI keeps its prompts, the data becomes one file, and the analysis that was
always done by hand gets a screen.

This page records the decisions and the order of work. The schema and the
migration mapping are on [the SQLite schema](sqlite-schema.md) page; the
tickable list of what each piece must do is
[Features and acceptance criteria](features.md).

## Why SQLite

The TOML store was chosen so a human could read and edit the log by hand. That
constraint has stopped paying: the log is 16 000 rows, edits are rare, and an
agent can write the CRUD code that a hand-editable file format was buying with
its slowness — sharded files, id bookkeeping across shards, a save path that
rewrites whole files, and a schema that only exists as documentation.

What SQLite buys back:

- **Integrity.** A foreign key from a set to its session and its exercise is
  enforced by the database, not by every query that happens to join.
- **One file to copy.** `gym sync` becomes a file transfer instead of a git
  history of a directory tree.
- **Queries the statistics need.** Trailing windows over weigh-ins and intake,
  group-by over muscle groups, ordered set membership — SQL does this, and the
  current `DirDb::find` does it by loading every file.
- **Concurrent readers.** The web UI and a CLI write can coexist under WAL.

What is lost, and accepted: no `diff` of the log, no hand edit, and a binary
file in place of readable text. The TOML-era data stays in git history and in
the current data directory as the archive.

## Decisions

| # | Decision | Consequence |
|---|----------|-------------|
| 1 | Places, muscle groups and food numbers get edit paths; they are no longer hand-edited TOML. | Add/create/edit/delete for every reference table, in the intervention tab and in the CLI. |
| 2 | PR detection uses the existing rep-factor table (the `to_one_rep_max` calculation). | Best estimated 1RM per exercise, compared against every earlier session of that exercise, independent of place. Sets above 30 reps are excluded (the table is bounded, and those are endurance sets). |
| 3 | A superset is n-ary and ordered, and its members share the load by definition — only the reps differ. | `set.superset` plus `set.position`; the migration synthesizes the partner row with the same load and the reps from the note. |
| 4 | The web UI is Leptos with hydration (SSR + WASM), and `install.sh` still installs one binary. | The hydrated site is embedded in the binary, so no site directory has to be deployed and the Android cross-build still produces something runnable. |
| 5 | `gym serve` binds loopback. | Default `127.0.0.1:8080`; a non-loopback address is only reachable if passed explicitly, and the documentation says what that exposes. |
| 6 | `gym sync` rsyncs the database to the server. | One destination in the config instead of a git remote; the copy is taken from a consistent snapshot, never from the live file. |
| 7 | The importer ships in this repository as `gym db import`. | It is tested like the rest of the program, and it doubles as the executable record of the old schema. One-off analysis scripts stay in the data directory. |
| 8 | `sqlx` against SQLite. | Async like the rest of the program, with `migrations/` applied by `sqlx::migrate!`. Queries are written at runtime (`sqlx::query`), not with the compile-time macros, so a build needs no database. |
| 9 | Supersets are represented as a group with ordered members, not as a pair. | A three-exercise circuit is the same shape as a pair, and the analysis can count the group once. |
| 10 | A deload halves the weight and halves the sets, and the analysis marks deload weeks on the chart rather than dropping them. | `session.deload`; the trend keeps the points and shades the week. |
| 11 | A PR is a better estimated 1RM than any earlier session of the exercise, by more than rounding. | Badge on the set; nothing stored, so a corrected set re-derives it. |
| 12 | Bulk/cut is one phase with a start date, shared by food and training. | `phase` row with kind, start, target weight and target rate; a session records the phase that was active, so history does not shift when a new phase starts. |
| 13 | Weight and intake statistics are computed over a trailing window with an uncertainty band, and the app reports "not enough data" rather than a number it cannot support. | See [weight and intake statistics](#weight-and-intake-statistics). |
| 14 | Exercise records are cleaned up: rename and merge, references re-pointed, then delete. | Typos are renamed, duplicates merged; whether two names are one movement is a reviewed pair, recorded in a mapping file. |

## The web UI

`gym serve` starts the server; the CLI prompts stay for the terminal. Six tabs,
two domains × three intents:

| Route | Tab | Contents |
|-------|-----|----------|
| `/food` | Food · daily | Record meals; record weight and body fat; today's totals; meal presets sorted by last use; the weight trend against the phase goal. |
| `/food/edit` | Food · intervention | Edit past meals and weigh-ins; edit a food's calories and macros; define and edit meal presets. |
| `/food/analysis` | Food · analysis | Weight and body fat trend overlaid on a stacked bar of intake, split by the food it came from; favourite foods ranked by frequency and by calories contributed. |
| `/gym` | Gym · daily | Record a session as today: place, exercises, load × reps. Effort presets beside the free-text note; a PR badge; the phase's target; the deload prompt when one is due; supersets; lists as a checklist. |
| `/gym/edit` | Gym · intervention | Edit past sessions and sets; exercise metadata (name, muscle groups and their amounts, an optional video link); places; muscle groups; rename and merge exercises; define lists. |
| `/gym/analysis` | Gym · analysis | Strength trend per body part by estimated 1RM; sets per muscle group over time; deload weeks shaded. |

### Daily use, in detail

- **Meal presets.** A named set of foods with amounts. Applying one records the
  meals in a single step; the list is ordered by last use, so the usual
  breakfast is the first entry.
- **Weight trend and the phase goal.** In a bulk the goal is the phase's target
  rate (default `+1 kg/month`); in a cut it is a rate at or below zero. The
  screen says whether the trend is inside the goal band, or that there is not
  enough data yet to say.
- **Maintenance estimate.** `intake + 7700 × Δweight / days` over the same
  window, only over days that were actually logged, with the same band. The
  difference between today's intake and that estimate is the number that
  answers "am I in a surplus".
- **Day inconsistency.** Days whose intake is far from the trend that the
  weight implies are listed for review. The app flags *days*, never a *food*:
  without a controlled repeat, a mis-valued food is not identifiable, and
  pointing at one would be a guess dressed as a finding.
- **Effort presets.** The note's most common values become one-tap presets
  (`fail`, `form fail`, `very hard`, `hard`, `incomplete`, `deload`), with free
  text still available beside them. The presets are a table, so they are
  editable.
- **Phase and deload targets.** In a bulk the suggested target is more load at
  the phase's rep target; in a cut, the same load at more reps. A deload week
  suggests half the load and half the sets.
- **Supersets.** Mark two or more exercises as one group in the session; the
  members keep their own load and reps, and the group is recorded once.
- **Lists.** An ordered set of exercises to work through, usable in any order
  (a squat rack being busy does not block the next item). The session stores
  the list it started from, so later list edits do not rewrite history.

### Weight and intake statistics

The numbers are small against the noise: a `+1 kg/month` bulk is about
`0.23 kg/week`, while a single weigh-in moves by up to `0.5 kg` between
mornings. So:

- **Trend** is a least-squares slope (equivalently a smoothed series) over a
  trailing window, default 21 days, requiring at least 14 weigh-ins in it.
  Below that the screen shows "not enough data" and the days it still needs.
- **Band** is the standard error of the slope, drawn with the trend. A goal is
  met when the whole band is inside the goal range, not when the point estimate
  is.
- **Maintenance** is computed only from days with real logging (a day with a
  single snack is not a day's intake) and only when at least 10 such days are in
  the window; otherwise it is withheld, because an unlogged dinner biases it
  downwards and the bias is indistinguishable from a slow metabolism.
- **Water weight** is handled by the window, not by a model: a switch between
  bulk and cut is excluded from the window for the first week of the new phase,
  and the phase's start date is what the analysis uses to know that.

## Sync

`gym sync` copies the database to the server with `rsync`. The copy is taken
from a consistent snapshot (`VACUUM INTO` a temporary file, then `rsync` that
file to the destination), because copying the live file while a writer is
active can capture a half-written page or a WAL that the receiver cannot apply.

The old git-based sync is retired; the data directory keeps its git history as
the archive of the TOML era. The configuration key `repo` becomes the `remote`
rsync destination.

## Order of work

Each step ships on its own, and each has a gate that must pass before the next
starts.

1. **Store and importer.** Schema, `gym db init`, `gym db import`, and the five
   existing commands on SQLite. Gate is parity, not new features: the same
   rows, and the same output for three food days, the history block for three
   exercises, and one weigh-in each way.
2. **`gym serve`, read-only.** The Leptos shell, the embedded hydrated assets,
   and the two analysis tabs reading only. This proves the build and install
   path before any write path exists.
3. **Food writes from the UI.** Daily recording, meal presets, weigh-ins.
4. **Gym writes from the UI.** Daily recording, supersets, effort presets,
   lists, and the intervention tab's edits and merges.
5. **Statistics.** Trend, band, maintenance, day inconsistency, phase goals.
6. **PR badges, deload reminders, and the video link.**

Cross-cutting from step 1: a `test/` fixture database, because the current
program has no tests at all and "the prompts still work" is otherwise an
opinion.

## Open

- **The server.** An rsync destination needs a concrete form (`user@host:path`
  or a mounted path), whether the copy is ever pulled back, and whether the
  server is only an archive or also the host that serves the UI.
- **The theme.** "Like the chat3 app" is not written down anywhere that can be
  followed; the palette, density and typography have to be stated before the
  shell is built, or the implementer reads that project's source.
