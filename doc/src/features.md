# Features and acceptance criteria

This page is the working list of what `gym` should do. Each entry states what
is true today, then the criteria that decide the entry is done. The criteria
are the editable part: rewrite them to describe the behaviour that is wanted,
and tick a box only when the behaviour is there and has been seen working.

To have a feature worked on, edit its criteria and name its heading to the
agent. Keep the `## Feature:` heading, the `Status:` line and the checkbox
list, so a reader can tell what is claimed from what is verified.

Status values: **works** (implemented and exercised), **partial** (some of it
exists), **missing** (nothing yet), **deferred** (wanted, deliberately not
now), **superseded** (replaced by a later entry or by
[the overhaul](overhaul.md)).

The first four entries are the behaviour the rebuild must preserve. They are
the parity gate: the rebuild is not done until they pass on the new store with
the real log in it.

## Feature: Log a training session

Status: works. `gym new` asks for place, time, description, then exercises and
their sets, printing the last three sessions of history for each exercise.
After the rebuild the prompts stay; the rows go into the database.

Acceptance criteria:

- [ ] A session is written with its place, date, phase and description.
- [ ] Each set is written immediately, with load, reps, note and its position
      in the session.
- [ ] Before the first set of an exercise, the last three sessions of that
      exercise at that place are printed as `date:` plus `load x rep` rows.
- [ ] An unknown exercise name is created instead of being refused.
- [ ] ESC on the `load` prompt ends the exercise; ESC on the exercise prompt
      ends the session.
- [ ] A session with no sets is still recorded, and is not printed as history.
- [ ] The previous session's numbers can be read at a glance without scrolling
      past more than three sessions of history.

## Feature: Log meals and see the day's totals

Status: works. `gym food` prints the current local day's calories, protein,
fat and carbohydrate before and after logging meals.

Acceptance criteria:

- [ ] Totals cover local midnight to local midnight, whatever the machine's
      timezone offset is.
- [ ] Each meal is written with food id, amount and description.
- [ ] The amount is a multiplier of the food's per-unit numbers.
- [ ] A food may be registered during the flow, with protein, fat and
      carbohydrate optionally unknown, and an unknown value is not stored as
      zero.
- [ ] Registering a name that already exists is refused with a message saying
      so.
- [ ] `Add more food?` defaults to no, ending the command with the updated
      totals.
- [ ] The totals are the same, to the cent, as the old binary's on the same
      data.

## Feature: Log weigh-ins

Status: works. `gym weight` asks for date, kilograms, body fat percentage and a
note.

Acceptance criteria:

- [ ] The weigh-in is written, and read back by the trend exactly once.
- [ ] Body fat may be left unknown without aborting the command, and unknown is
      distinguishable from a measured zero.
- [ ] The note may be empty.
- [ ] Two weigh-ins on the same day are both kept, in the order they were
      entered.

## Feature: Build and install

Status: works. `./install.sh` formats, builds a release binary and installs it
into `$CARGO_HOME/bin` atomically; `./deploy.sh` cross-compiles for
`aarch64-linux-android`.

Acceptance criteria:

- [ ] `./install.sh` is idempotent and safe while `gym` is running.
- [ ] The installed binary reports which build it is (`gym --version`).
- [ ] The hydrated web assets are inside the binary, so installing is still one
      file and the Android build produces something runnable.
- [ ] Every query is a `sqlx` macro checked at build time; a dynamic SQL string
      is absent from the source tree.
- [ ] A build needs no database: the `.sqlx` offline cache is committed and
      `install.sh` builds with `SQLX_OFFLINE=true`.
- [ ] After a migration changes, one documented command sequence refreshes the
      cache, and a stale cache fails the build instead of failing at runtime.
- [ ] `./deploy.sh` states its NDK prerequisites and fails with a message, not
      a linker error, when they are missing.
- [ ] `mdbook build doc` is checked, and a broken internal link fails it.

## Feature: The SQLite store and the importer

Status: missing. The log is TOML files sharded by day and month; the rebuild
puts it in one SQLite database.

Acceptance criteria:

- [ ] `gym db init` creates an empty database at the configured path with the
      schema and the seeded effort vocabulary, and refuses to touch a file that
      is already a database.
- [ ] The schema exists only as `migrations/0001_init.sql`, applied by
      `sqlx::migrate!` on open, with no second copy in the source.
- [ ] `gym db import <toml-dir>` fills a fresh database from the TOML
      directories and is all-or-nothing.
- [ ] Every old row arrives with its old id, and every reference — a set's
      session, a meal's food, an exercise's muscle groups — points at the row
      with the same content.
- [ ] A set that no session lists is reported and stops the import, rather than
      being attached to a guess.
- [ ] The 226 parseable superset notes become groups of two sets sharing a
      load, with the partner row marked `inferred`; the unparseable ones are
      listed for a human.
- [ ] The row counts and the aggregates listed under *Verifying an import* hold
      on the real log.
- [ ] The schema is documented on [the schema page](sqlite-schema.md), and the
      document matches the migration files.

## Feature: `gym web`, the local web UI

Status: missing. The command parses an address and panics with
`not yet implemented`. It keeps its name — `web` is what exists and what is
shortest.

Acceptance criteria:

- [ ] `gym web` serves the UI on `127.0.0.1:8080` by default, and binds
      loopback unless another address is passed explicitly.
- [ ] The UI is Leptos with hydration: the first paint is server-rendered, and
      the page works with scripts enabled and without them.
- [ ] A new install never pairs a cached script with a newly built WASM: every
      response carries `Cache-Control: no-cache`.
- [ ] The six tabs exist at stable routes: `/food`, `/food/edit`,
      `/food/analysis`, `/gym`, `/gym/edit`, `/gym/analysis`.
- [ ] The look is the one on [the overhaul](overhaul.md#the-look) page: black
      background, green ramp, blocky 2px borders, no rounded corners, Tailwind
      defaults for type.
- [ ] The analysis tabs are read-only: no request from them writes a row.
- [ ] The server and a CLI write can run at the same time without a lock
      failure under normal use (WAL, a busy timeout, one short transaction per
      row, never a transaction held across a prompt).
- [ ] A page reflects a change made by another process on the next request,
      with no restart.

## Feature: Food · daily

Status: missing. Today's `gym food` prompts, on a screen.

Acceptance criteria:

- [ ] Meals can be recorded for a chosen time with a food, an amount and a
      note, and today's totals update in place.
- [ ] A food can be registered from the same screen, and its macros may be
      unknown.
- [ ] A meal preset records all of its foods in one action, and the preset list
      is ordered by last use.
- [ ] A weigh-in can be recorded from the same screen, with body fat optional.
- [ ] The screen shows the weight trend against the current phase's goal, or
      says what data is still missing.
- [ ] The screen shows today's intake against the estimated maintenance, with
      the estimate withheld when the window has too few logged days.

## Feature: Weight and intake statistics

Status: missing. Nothing computes a trend; a weigh-in is only ever read back by
the exercise history.

Acceptance criteria:

- [ ] The trend is a least-squares slope over a trailing window (21 days by
      default) that requires at least 14 weigh-ins, and the screen shows how
      many are missing when there are fewer.
- [ ] The trend is drawn with its uncertainty band, and a goal is met only when
      the whole band is inside the goal range.
- [ ] The maintenance estimate uses intake and weight change over the same
      window, counts only days with real logging, and is withheld below 10 such
      days.
- [ ] A bulk's default goal is `+1 kg/month`; a cut's is at or below zero; both
      come from the phase, not from a second setting.
- [ ] Days inconsistent with the weight-implied trend are listed; no food is
      ever blamed from intake alone.
- [ ] The first week of a new phase is excluded from the window, so a switch
      between a bulk and a cut does not read as a jump in body composition.
- [ ] A test with synthetic weigh-ins of known slope and noise recovers the
      slope within its band, and refuses the cases below the data threshold.

## Feature: Food · intervention

Status: missing. Past meals and weigh-ins can only be edited by hand in the TOML
files, and a food's numbers cannot be changed at all once registered.

Acceptance criteria:

- [ ] A past meal or weigh-in can be found by date, edited field by field, and
      deleted; the day's totals are correct afterwards.
- [ ] A food's calories, protein, fat, carbohydrate and description can be
      changed after it was registered.
- [ ] A food that a meal still refers to cannot be deleted, and the refusal
      says how many meals use it.
- [ ] Meal presets can be created, edited and deleted, with their foods and
      amounts.
- [ ] Every edit affects only the row it names.

## Feature: Food · analysis

Status: missing.

Acceptance criteria:

- [ ] A chart shows weight and body fat as lines over a stacked bar of daily
      intake, split by the food that contributed it.
- [ ] The same chart marks deload weeks and phase boundaries.
- [ ] Foods are ranked by how often they are eaten and by how much of the
      intake they contribute.
- [ ] A range can be chosen, and the chart states the range and the number of
      logged days in it.

## Feature: Gym · daily

Status: partial. `gym new` records a session; effort, PRs, phases, deloads,
supersets and lists do not exist.

Acceptance criteria:

- [ ] A session is recorded as today's prompts record it, from one screen.
- [ ] The note carries an effort picked from the presets, with free text still
      available, and both are stored.
- [ ] A set that beats the best estimated 1RM of every earlier session of that
      exercise is marked as a PR, using the existing rep-factor table, ignoring
      sets above 30 reps.
- [ ] The phase's target is shown beside the last performance: more load in a
      bulk, the same load at more reps in a cut.
- [ ] A deload is prompted when one is due, and a deload session's suggested
      target is half the load and half the sets.
- [ ] Two or more exercises can be recorded as one superset group, each with
      its own reps, sharing the load.
- [ ] A list can be started as the session's checklist, items can be completed
      in any order, and the session keeps the list it started from after the
      list itself is edited.

## Feature: Gym · intervention

Status: partial. Exercises can be created with a name only; places and muscle
groups have no editing path, and a duplicate exercise can never be retired.

Acceptance criteria:

- [ ] Past sessions and sets can be found, edited and deleted, and a session's
      set positions stay consistent afterwards.
- [ ] A place and a muscle group can be created, renamed and deleted; deleting
      one that is still referenced is refused with the count.
- [ ] An exercise's muscle groups and their amounts can be assigned and
      changed.
- [ ] An exercise can be renamed, and a set of exercises can be merged into one,
      with every set and list item re-pointed and the merged-away rows deleted.
- [ ] A merge of two exercises that share a muscle group keeps one amount, and
      the surviving value is visible before the merge is confirmed.
- [ ] An exercise can carry a video link, and the link is validated and shown
      on the daily screen.
- [ ] Lists of exercises can be created, reordered and deleted.

## Feature: Gym · analysis

Status: missing.

Acceptance criteria:

- [ ] Strength is plotted per body part, from the best estimated 1RM per
      session, one line per exercise.
- [ ] Sets per muscle group are plotted over time, counting a superset group's
      contribution once per member exercise.
- [ ] Deload weeks are shaded rather than dropped, so the trend keeps its
      points.
- [ ] Inferred rows from the migration are included and marked as inferred.
- [ ] Exercises with no muscle group are listed, so an empty analysis is
      explained rather than silent.

## Feature: Sync the database to the remote

Status: missing. Today's `gym sync` is a git pull, commit and push of the data
directory.

Acceptance criteria:

- [ ] `gym sync` copies the database to `remote` with `rsync`, from a
      consistent snapshot taken with `VACUUM INTO`, never from the live file.
- [ ] `remote` is one string used verbatim as rsync's destination, and a local
      path and a `host:path` both work.
- [ ] Nothing is ever pulled back: a sync only writes at the destination.
- [ ] A failed transfer exits non-zero and says what failed; a successful one
      says where the copy went and how big it is.
- [ ] Running it twice in a row succeeds and leaves one file at the
      destination.
- [ ] The copy opens with `PRAGMA integrity_check` returning `ok`.
- [ ] `gym sync` works while the server and a CLI write are running.

## Feature: Tests

Status: missing. The program has no tests.

Acceptance criteria:

- [ ] `test/` holds a fixture database with a known food day, a known exercise
      history and a known set of weigh-ins.
- [ ] The parity checks — totals to the cent, the history block, the weigh-in
      read-back — are automated against the fixture.
- [ ] The statistics are tested against synthetic series with a known slope and
      noise, including the "not enough data" cases.
- [ ] `cargo test` runs them, and a failure names the assertion rather than a
      diff of a whole table.

## Feature: Non-interactive input

Status: deferred. Every command prompts; nothing can be scripted. The web UI
covers the day-to-day, so this waits until there is a reason to drive the CLI
from a script.

Acceptance criteria:

- [ ] Each logging command accepts its answers as flags or on stdin, and writes
      the same rows the prompts would.
- [ ] The default remains interactive: no flags means prompts.
- [ ] A malformed value exits non-zero with a message naming the field and the
      value, and writes nothing.

## Feature: A data directory a human owns

Status: superseded. The TOML store was chosen so the log could be read and
edited by hand; the rebuild replaces it with SQLite, for the reasons on
[the overhaul](overhaul.md#why-sqlite) page. The TOML-era directories stay as
the archive.

## Feature: Bootstrap a fresh data directory

Status: superseded by [the SQLite store and the importer](#feature-the-sqlite-store-and-the-importer).
The old store needed four index files and four directories, each with at least
an empty table, before any command worked.

## Feature: Correct a logged row

Status: superseded by [Food · intervention](#feature-food--intervention) and
[Gym · intervention](#feature-gym--intervention). The CLI is not getting
editing commands; the intervention tabs do that.

## Feature: Read the log back

Status: superseded by the two analysis tabs,
[Food · analysis](#feature-food--analysis) and
[Gym · analysis](#feature-gym--analysis).

## Feature: Exercise and food metadata

Status: superseded by [Gym · intervention](#feature-gym--intervention) and
[Food · intervention](#feature-food--intervention), which cover editing a food's
numbers, an exercise's muscle groups, places and muscle groups.

## Feature: Sync the data directory to a remote

Status: superseded by [Sync the database to the remote](#feature-sync-the-database-to-the-remote).
The git transport is retired; the data directory keeps its history as the
archive of the TOML era.

## Feature: Web view

Status: superseded by [`gym web`, the local web UI](#feature-gym-web-the-local-web-ui).
