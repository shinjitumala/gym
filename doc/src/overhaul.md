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
| 5 | `gym web` binds loopback, and keeps the name it has today. | Default `127.0.0.1:8080`; a non-loopback address is only reachable if passed explicitly, and the documentation says what that exposes. There is no `gym serve`: the command that already exists does the job. |
| 6 | `gym sync` rsyncs the database to the remote, whose whole configuration is its path. | The config key `repo` becomes `remote`, one string used verbatim as the rsync destination — a local path or `host:path`, rsync's own syntax either way. The copy is taken from a consistent snapshot, never from the live file, and the transfer is one-way. |
| 7 | The importer ships in this repository as `gym db import`. | It is tested like the rest of the program, and it doubles as the executable record of the old schema. One-off analysis scripts stay in the data directory. |
| 8 | `sqlx` against SQLite, **with the compile-time macros only**. | `sqlx::query!`, `query_as!` and `query_scalar!` check every statement against the migrated schema at build time, so a column rename is a compile error. The `.sqlx` offline cache is committed and builds run with `SQLX_OFFLINE=true`, so neither the installer nor a cross-build needs a database. Hand-built SQL strings (`sqlx::query(&format!(...))`) are not permitted; a filter that varies is written as one statement with optional parameters (`WHERE (?1 IS NULL OR date >= ?1)`). |
| 9 | Supersets are represented as a group with ordered members, not as a pair. | A three-exercise circuit is the same shape as a pair, and the analysis can count the group once. |
| 10 | A deload is opted into, never entered automatically. A reminder appears once the configured number of weeks has passed since the last deload, and a deload session halves the weight and halves the sets. The analysis marks deload weeks on the chart rather than dropping them. | `session.deload` is only ever set by the person logging the session; `setting.deload_weeks` (default 6) decides when the reminder appears; the trend keeps the points and shades the week. |
| 11 | A PR is a better estimated 1RM than any earlier session of the exercise, by more than rounding. | Badge on the set; nothing stored, so a corrected set re-derives it. |
| 12 | Bulk/cut is one phase with a start date, shared by food and training. | `phase` row with kind, start, target weight and target rate; a session records the phase that was active, so history does not shift when a new phase starts. |
| 13 | Weight and intake statistics are computed over a trailing window with an uncertainty band, and the app reports "not enough data" rather than a number it cannot support. | See [weight and intake statistics](#weight-and-intake-statistics). |
| 14 | Exercise records are cleaned up: rename and merge, references re-pointed, then delete. | Typos are renamed, duplicates merged; whether two names are one movement is a reviewed pair, recorded in a mapping file. |

## The web UI

`gym web` starts the server — the command exists today and panics, so the name
and the address flag are already there to fill in. The CLI prompts stay for the
terminal. Six tabs, two domains × three intents:

| Route | Tab | Contents |
|-------|-----|----------|
| `/food` | Food · daily | Record meals; record weight and body fat; today's totals; meal presets sorted by last use; the weight trend against the phase goal. |
| `/food/edit` | Food · intervention | Edit past meals and weigh-ins; edit a food's calories and macros; define and edit meal presets. |
| `/food/analysis` | Food · analysis | Weight and body fat trend overlaid on a stacked bar of intake, split by the food it came from; favourite foods ranked by frequency and by calories contributed. |
| `/gym` | Gym · daily | Record a session as today: place, exercises, load × reps. Effort presets beside the free-text note; a PR badge; the phase's target; the deload reminder and its opt-in; supersets; lists as a checklist. |
| `/gym/edit` | Gym · intervention | Edit past sessions and sets; exercise metadata (name, muscle groups and their amounts, an optional video link); places; muscle groups; rename and merge exercises; define lists. |
| `/gym/analysis` | Gym · analysis | Strength trend per body part by estimated 1RM; sets per muscle group over time; deload weeks shaded. |

### The look

The shell follows the chat3 app: near-black background, a green ramp, blocky
controls, and no rounded corners anywhere. Concretely, with Tailwind v4 (a
single `@import "tailwindcss"`, classes on the markup, no theme file):

| Role | Class |
|------|-------|
| Page background | `bg-black` |
| Body text | `text-gray-400`, base size `text-sm` |
| Primary text, bold values, header title | `text-green-400`, `font-bold`, `tracking-wide` on the title |
| Section headings (`h2`), small labels | `text-green-500` |
| Input text | `text-green-300` (`input`), `text-green-200` (`textarea`) |
| Every border, every divider | `border-green-900`, 2px on controls and the header rule, 1px on section separators |
| Hover and selected background | `bg-green-950`, with `border-green-500` |
| De-emphasized meta | `text-gray-600`, `text-xs` |
| Errors only | `border-red-500`, `bg-red-950`, `text-red-300` |

Geometry and structure:

- `rounded-none` on every control; buttons and inputs carry `border-2`, `px-1`
  or `px-2`, `hover:border-green-500 hover:bg-green-950`, and
  `disabled:opacity-30 disabled:cursor-not-allowed`.
- The app is one column: `h-dvh flex flex-col overflow-hidden`, spacing in
  `gap-1`/`gap-2`/`p-1`/`p-2` steps, popups `fixed` and centred with `border-2`
  and `shadow-lg`.
- A `Header` component (title left, controls right, `border-b-2` `bg-black`) and
  a `Section` component (`h2` in `text-green-500`, separated by `border-t` with
  `first:border-t-0`) — the two idioms the tabs are built from.
- Control rows collapse to one column below `md`, two above (`md:grid-cols-2`).
- Native controls keep the palette: `accent-green-500` on checkboxes and radios,
  and a thin scrollbar (`scrollbar-thin`, green-900 thumb on black).
- Fonts are Tailwind's defaults; nothing is imported.

Two hazards come with hydration, and both are already solved in that app:

- **Cache the bundle carefully.** The bundle's URLs are stable across builds, so
  a browser that pairs a cached script with a new WASM walks two different DOM
  shapes and panics inside hydration. Every response carries
  `Cache-Control: no-cache`, which still allows a 304 and never lets half of a
  previous build survive an install.
- **Serve the assets from inside the binary.** The stylesheet and the WASM are
  embedded and served at `/pkg/…`, so an install is still one file.

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
  the phase's rep target; in a cut, the same load at more reps. A deload session
  suggests half the load and half the sets.
- **The deload reminder.** When `deload_weeks` (default 6) has passed since the
  last deload session, the daily screen says so and offers to enter deload mode.
  Nothing switches by itself: the person opts in, the session is flagged, and the
  suggested targets become half the load and half the sets for that session only.
  Declining leaves the reminder in place; the week count starts again from the
  deload that is actually logged.
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
  trailing window, requiring a minimum number of weigh-ins in it. Below that the
  screen shows "not enough data" and how many are still needed.
- **Band** is the standard error of the slope, drawn with the trend. A goal is
  met when the whole band is inside the goal range, not when the point estimate
  is.
- **Maintenance** is computed only from days with real logging (a day with a
  single snack is not a day's intake) and only once enough such days are in the
  window; otherwise it is withheld, because an unlogged dinner biases it
  downwards and the bias is indistinguishable from a slow metabolism.
- **Water weight** is handled by the window, not by a model: a switch between
  bulk and cut is excluded from the window for the first week of the new phase,
  and the phase's start date is what the analysis uses to know that.

The window and its thresholds are settings, not constants, so they can be
changed without a rebuild; they are listed below.

### Settings

Four numbers are policy rather than data:

| Key | Default | Decides |
|-----|---------|---------|
| `deload_weeks` | `6` | When the deload reminder appears. |
| `trend_window_days` | `21` | The trailing window for the weight trend and maintenance. |
| `trend_min_weighins` | `14` | Weigh-ins required in the window before a slope is shown. |
| `maintenance_min_days` | `10` | Logged days required in the window before maintenance is shown. |

They live in the database's `setting` table, not in `gym.toml`, because they
are properties of the analysis rather than of the machine, because changing one
should not mean editing a file by hand, and because a slope shown in the past can
then be recomputed with the window that was in force. Everything else is either
derived or a row.

## Sync

`gym sync` copies the database to the remote with `rsync`, from the path the
config calls `remote`. The copy is taken from a consistent snapshot
(`VACUUM INTO` a temporary file, then `rsync` that file to the destination),
because copying the live file while a writer is active can capture a
half-written page or a WAL that the receiver cannot apply.

The configuration is one string, used verbatim as rsync's destination:

```json
{
  "db": "/home/me/g/gym_data/gym.db",
  "remote": "fpr0:/home/me/g/gym_data/gym.db"
}
```

A local path works the same way (`/mnt/backup/gym.db`), since rsync takes either.
The transfer is one-way: nothing is ever pulled back over it, so a copy on the
remote is an archive and never a second writer.

The old git-based sync is retired; the data directory keeps its git history as
the archive of the TOML era.

## What shipped first

Step 1 is in place: the schema is `migrations/0001_init.sql`, the store is
`sqlx` with compile-time-checked macros, `gym db init` and `gym db import`
exist, and `gym weight`, `gym new`, `gym food` and `gym sync` read and write
the database. `test/` holds the store, importer and command-surface tests.
The configuration moved from JSON to TOML (`db` is now a file path, `repo`
became `remote`), and the old `act` crate — the git pull/commit/push scripts —
was removed with the git transport.

## Order of work

Each step ships on its own, and each has a gate that must pass before the next
starts.

1. **Store and importer. Done.** Schema and migrations, `gym db init`,
   `gym db import`, and the five existing commands on SQLite, with parity
   checked against the real archive: identical ids, membership and totals.
   The `.sqlx` offline cache and `SQLX_OFFLINE=true` in the installer landed
   here, because from this step on no build has a database to point at. The
   retired git sync left with it: the `act` crate is gone, and `gym sync` is
   the rsync described below.
2. **`gym web`, read-only.** The Leptos shell in the chat3 look, the embedded
   hydrated assets, and the two analysis tabs reading only. This proves the
   build and install path before any write path exists.
3. **Food writes from the UI.** Daily recording, meal presets, weigh-ins.
4. **Gym writes from the UI.** Daily recording, supersets, effort presets,
   lists, and the intervention tab's edits and merges.
5. **Statistics.** Trend, band, maintenance, day inconsistency, phase goals.
6. **PR badges, deload reminders, and the video link.**

Cross-cutting from step 1: a `test/` fixture database, because the current
program has no tests at all and "the prompts still work" is otherwise an
opinion.

## Refreshing the compile-time queries

The macros read the schema, so the schema and the checked queries are refreshed
together, and a change to one is a build failure until the other follows:

```bash
$ DATABASE_URL=sqlite:///tmp/gym-schema.db sqlx database create
$ DATABASE_URL=sqlite:///tmp/gym-schema.db sqlx migrate run
$ DATABASE_URL=sqlite:///tmp/gym-schema.db cargo sqlx prepare -- --all-targets
```

`cargo sqlx prepare` writes `.sqlx/`, which is committed. After that,
`SQLX_OFFLINE=true cargo build` — what `install.sh` runs — needs no database,
and so does the Android cross-build.

## Open

Nothing is outstanding from the design decisions. The three items that were open
are settled: the sync configuration is the `remote` path, the look is written
down above, and the deload trigger is the configured number of weeks, with deload
mode entered by opting into it. What remains undecided belongs to implementation,
not design: whether `gym web` also serves over the LAN (it can, by passing an
address, and the docs say what that exposes).
