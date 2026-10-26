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

Status: works, on the database. `gym new` asks for place, time, description,
then exercises and their sets, printing the last three sessions of history for
each exercise.

Acceptance criteria:

- [x] A session is written with its place, date, phase and description.
- [x] Each set is written immediately, with load, reps, note and its position
      in the session.
- [x] Before the first set of an exercise, the last three sessions of that
      exercise at that place are printed as `date:` plus `load x rep` rows.
- [x] An unknown exercise name is created instead of being refused.
- [x] ESC on the `load` prompt ends the exercise; ESC on the exercise prompt
      ends the session.
- [x] A session with no sets is still recorded, and is not printed as history.
- [ ] The previous session's numbers can be read at a glance without scrolling
      past more than three sessions of history.

## Feature: Log meals and see the day's totals

Status: works, on the database. `gym food` prints the current local day's calories, protein,
fat and carbohydrate before and after logging meals.

Acceptance criteria:

- [x] Totals cover local midnight to local midnight, whatever the machine's
      timezone offset is.
- [x] Each meal is written with food id, amount and description.
- [x] The amount is a multiplier of the food's per-unit numbers.
- [x] A food may be registered during the flow, with protein, fat and
      carbohydrate optionally unknown, and an unknown value is not stored as
      zero.
- [x] Registering a name that already exists is refused with a message saying
      so.
- [x] `Add more food?` defaults to no, ending the command with the updated
      totals.
- [x] The totals are the same, to the cent, as the old binary's on the same
      data.

## Feature: Log weigh-ins

Status: works, on the database. `gym weight` asks for date, kilograms, body fat percentage and a
note.

Acceptance criteria:

- [x] The weigh-in is written, and read back by the trend exactly once.
- [x] Body fat may be left unknown without aborting the command, and unknown is
      distinguishable from a measured zero.
- [x] The note may be empty.
- [x] Two weigh-ins on the same day are both kept, in the order they were
      entered.

## Feature: Build and install

Status: works, and the build is offline: `.sqlx` carries the checked queries.
`./install.sh` formats, builds a release binary and installs it into
`$CARGO_HOME/bin` atomically; `./deploy.sh` cross-compiles for
`aarch64-linux-android`.

Acceptance criteria:

- [ ] `./install.sh` is idempotent and safe while `gym` is running.
- [ ] The installed binary reports which build it is (`gym --version`).
- [ ] The hydrated web assets are inside the binary, so installing is still one
      file and the Android build produces something runnable.
- [x] Every query is a `sqlx` macro checked at build time; a dynamic SQL string
      is absent from the source tree.
- [x] A build needs no database: the `.sqlx` offline cache is committed and
      `install.sh` builds with `SQLX_OFFLINE=true`.
- [x] After a migration changes, one documented command sequence refreshes the
      cache, and a stale cache fails the build instead of failing at runtime.
- [ ] `./deploy.sh` states its NDK prerequisites and fails with a message, not
      a linker error, when they are missing.
- [ ] `mdbook build doc` is checked, and a broken internal link fails it.

## Feature: The SQLite store and the importer

Status: works. The log lives in one SQLite file, and `gym db import` fills it
from the TOML archive the previous version wrote.

Acceptance criteria:

- [x] `gym db init` creates an empty database file at the configured path with
      the schema and the seeded tables, and refuses to touch a file that is
      already a database.
- [x] `db` in the config is the path to that file; a path that is a directory is
      refused with a message saying so rather than being opened.
- [x] The schema exists only as `migrations/0001_init.sql`, applied by
      `sqlx::migrate!` on open, with no second copy in the source.
- [x] `gym db import --dir <archive>` fills a fresh database from the TOML
      directories and is all-or-nothing.
- [x] Every old row arrives with its old id, and every reference — a set's
      session, a meal's food, an exercise's muscle groups — points at the row
      with the same content.
- [x] A set that no session lists is reported and stops the import, rather than
      being attached to a guess; so is a set two sessions both list.
- [x] The parseable superset notes become groups of two sets sharing a load,
      with the partner row marked `inferred`; the unparseable ones are listed
      for a human. With the 2026-05 archive and its mapping: 214 rows from 214
      notes, 13 reported.
- [x] The row counts and the aggregates listed under *Verifying an import* hold
      on the real archive.
- [x] The schema is documented on [the schema page](sqlite-schema.md), and the
      document matches the migration file.
- [x] `setting` is seeded with the four defaults and read through a typed
      accessor, so an absent row means the documented default rather than an
      error.

## Feature: `gym web`, the local web UI

Status: works. `gym web` serves the three modes on loopback, each with its daily,
edit and analyse pages, and every one of them writes. The scenarios it is judged
against are on [the requirements page](requirements.md).

Acceptance criteria:

- [x] `gym web` serves the UI on `127.0.0.1:8080` by default, and binds
      loopback unless another address is passed explicitly.
- [x] The UI is Leptos with hydration: the first paint is server-rendered, and
      the page works with scripts enabled and without them.
- [x] A new install never pairs a cached script with a newly built WASM: every
      response carries `Cache-Control: no-cache`.
- [x] The bar is the three modes — `/gym`, `/food`, `/weight` — and a mode's
      `/edit` and `/analyse` pages are reached from buttons at the bottom of its
      daily page.
- [x] Every page writes what it is for; no page is read-only by omission.
- [x] The look is the one on [the overhaul](overhaul.md#the-look) page: black
      background, green ramp, blocky 2px borders, no rounded corners.
- [x] The analysis pages are read-only: no request from them writes a row.
- [x] The server and a CLI write can run at the same time without a lock
      failure under normal use (WAL, a busy timeout, one short transaction per
      row, never a transaction held across a prompt).
- [x] A page reflects a change made by another process on the next request,
      with no restart.

## Feature: Food · daily

Status: works. The meal being entered, today's totals against maintenance, and
the day's meals.

Acceptance criteria:

- [x] A meal is built as rows from a searchable food list sorted by the last
      time each food was eaten, previewed with its own calories and macros, and
      saved in one transaction; nothing is written before `save`.
- [x] A food can be registered from the edit tab, and its macros may be
      unknown.
- [x] A meal preset records all of its foods in one action, and the preset list
      is ordered by last use.
- [x] A weigh-in has its own tab, with the time defaulting to now and body fat
      optional.
- [x] The screen shows today's intake against the estimated maintenance, with
      the estimate withheld when the window has too few logged days.

## Feature: Weight and intake statistics

Status: works. `src/stats.rs` is the arithmetic, with its own tests; the weight
pages print its verdicts.

Acceptance criteria:

- [x] The trend is a least-squares slope over a trailing window whose length,
      the weigh-in minimum and the logged-day minimum all come from the
      `setting` table, and the page shows how many weigh-ins are missing when
      there are too few.
- [x] The trend is drawn with its uncertainty band, and a goal is met only when
      the whole band is inside the goal range.
- [x] The maintenance estimate uses intake and weight change over the same
      window, counts only days with real logging, and is withheld below the
      configured minimum.
- [x] Changing a setting takes effect on the next request, with no restart and
      no rebuild.
- [x] A bulk's goal is the phase's target rate, and a cut's is at or below zero;
      both come from the phase, not from a second setting.
- [x] Days inconsistent with the weight-implied trend are listed; no food is
      ever blamed from intake alone.
- [ ] The first week of a new phase is excluded from the window, so a switch
      between a bulk and a cut does not read as a jump in body composition.
- [x] A test with synthetic weigh-ins of known slope and noise recovers the
      slope within its band, and refuses the cases below the data threshold.

## Feature: Food · intervention

Status: works. Meals, weigh-ins, foods and presets, each edited in place.

Acceptance criteria:

- [x] A past meal or weigh-in can be found by date, edited field by field, and
      deleted; the day's totals are correct afterwards.
- [x] A food's calories, protein, fat, carbohydrate and description can be
      changed after it was registered.
- [x] A food that a meal still refers to cannot be deleted, and the refusal
      says how many meals use it.
- [x] Meal presets can be created and deleted, with their foods and amounts;
      editing one in place is still missing.
- [x] Every edit affects only the row it names.

## Feature: Weight · daily

Status: works. The trend, the last weigh-in and the form.

Acceptance criteria:

- [x] A weigh-in is a date, a time defaulting to now, kilograms, optional body
      fat and an optional note, saved from one screen.
- [x] The last weigh-in, its body fat and the trend against the phase's goal are
      shown, or the page says how many weigh-ins are still missing.
- [x] The recent weigh-ins are listed newest first.

## Feature: Food · analysis

Status: superseded by [Weight · analysis](#feature-weight--analysis). The intake
chart is read against the weight trend, so it is drawn under it rather than on a
page of its own, and the per-food view is the range's ranking under that chart:
food keeps no analysis page, so the ranking moved beside the bars instead of
returning as one. What that decision buys is on
[the requirements page](requirements.md#the-per-food-ranking-has-no-page-of-its-own).

## Feature: Weight · analysis

Status: works. Weight and body fat over a chosen range, with the intake bars
underneath, the range's foods ranked beside them, and the deload weeks shaded.

Acceptance criteria:

- [x] The weight line is coloured by the phase each weigh-in was taken in, with
      bulk and cut in different colours and a legend saying which is which.
- [x] Body fat is drawn on its own when at least two weigh-ins measured it.
- [x] Below the line, one bar per day, stacked by the food that contributed it,
      switchable between calories and each macro.
- [x] Pressing or hovering a bar names the foods that made that day, with what
      each contributed.
- [x] A range can be chosen, and the page states the range and how many of its
      days count as logged.
- [x] The chart marks deload sessions and phase boundaries: the weight line is
      coloured per phase, and the deload weeks are shaded behind it.

## Feature: Gym · daily

Status: works. A session is started from a place, filled from a preset or from
the exercise list, recorded one exercise at a time through the exercise window,
and ended; a deload is marked on the session and a superset is recorded as one
group. Inferred rows from the migration are still missing from the analysis.

Acceptance criteria:

- [x] A session starts from a place picked from a list sorted by the last
      session there, and only one session is open at a time.
- [x] A preset fills the checklist in its own order, and any exercise on it opens
      the exercise window.
- [x] The exercise window defaults the load to the last used and the effort to
      the last recorded, offers `fail`, `form fail` and `hard` with what each
      means, shows the last three sessions, and prints what to attempt next.
- [x] Saving a row stamps the time it was saved, and X returns to the checklist
      with the exercise marked done.
- [x] End session closes it, and a session with no sets is discarded.
- [x] The effort of a set is stored as a row and compared before load and reps
      when the next set is advised; `src/train.rs` holds that rule and its tests.
- [ ] A set that beats the best estimated 1RM of every earlier session of that
      exercise is marked as a PR, using the existing rep-factor table, ignoring
      sets above 30 reps.
- [ ] The phase's target is shown beside the last performance: more load in a
      bulk, the same load at more reps in a cut.
- [x] A deload reminder appears once `deload_weeks` has passed since the last
      deload session, and the week count and the reminder are both visible.
- [x] Entering deload mode is an explicit opt-in on the session; nothing switches
      by itself, and declining leaves the reminder in place.
- [x] A deload session's suggested target is half the load and half the sets,
      drawn from the last session trained normally, for that session only: the
      deload's rows are shown in the history but are not the numbers to beat.
- [x] Two or more exercises can be recorded as one superset group, each with
      its own reps, sharing the load.
- [x] A preset can be applied as the session's checklist, items can be completed
      in any order, and the checklist is stored per session, so editing the
      preset later does not rewrite a finished session.

## Feature: Gym · intervention

Status: works. Sessions and sets, exercises with their muscle groups, gym
presets, places and muscle groups are all editable.

Acceptance criteria:

- [x] Past sessions and sets can be found, edited and deleted, and a session's
      set positions stay consistent afterwards.
- [x] A place and a muscle group can be created, renamed and deleted; deleting
      one that is still referenced is refused with the count.
- [x] An exercise's muscle groups and their amounts can be assigned and
      changed.
- [x] An exercise can be renamed, and a set of exercises can be merged into one,
      with every set re-pointed and the merged-away row deleted.
- [ ] A merge of two exercises that share a muscle group keeps one amount, and
      the surviving value is visible before the merge is confirmed.
- [x] An exercise can carry a video link, and it is validated on save.
- [x] Gym presets (a named list of exercises) can be created and deleted, and
      applying one fills a session's checklist.

## Feature: Gym · analysis

Status: works. One exercise's strength overlaid with sets per week for it and
for the exercises that share its main muscle group.

Acceptance criteria:

- [x] Strength is plotted for one chosen exercise, from its best estimated 1RM
      per session, with its body part named.
- [x] Sets per week are counted for the chosen exercise and for every exercise
      that shares its main muscle group, and drawn on a second scale over the
      strength line, so the question "did strength move when the volume did" is
      one chart.
- [x] The time range can be changed.
- [x] Deload weeks are shaded rather than dropped, so the trend keeps its
      points.
- [ ] Inferred rows from the migration are included and marked as inferred.
- [x] Exercises with no muscle group are listed, so an empty analysis is
      explained rather than silent.

## Feature: Sync the database to the remote

Status: works. `gym sync` snapshots the database and rsyncs it to `remote`.

Acceptance criteria:

- [x] `gym sync` copies the database to `remote` with `rsync`, from a
      consistent snapshot taken with `VACUUM INTO`, never from the live file.
- [x] `remote` is one string used verbatim as rsync's destination, and a local
      path and a `host:path` both work.
- [x] Nothing is ever pulled back: a sync only writes at the destination.
- [x] A failed transfer exits non-zero and says what failed; a successful one
      says where the copy went and how big it is.
- [x] Running it twice in a row succeeds and leaves one file at the
      destination.
- [x] The copy opens with `PRAGMA integrity_check` returning `ok`.
- [ ] `gym sync` works while the server and a CLI write are running.

## Feature: Tests

Status: works for the store, the importer, the command surface, the statistics
and the recommendation rule; `cargo test` runs them. The rendered UI is walked by
`python3 test/web/drive.py`, one scenario per requirement.

Acceptance criteria:

- [x] `test/` holds fixtures built per test: a known food day, a known exercise
      history, known weigh-ins, and a small TOML archive for the importer.
- [x] The parity checks — totals to the cent, the history block, the weigh-in
      read-back — are automated.
- [x] The statistics are tested against synthetic series with a known slope and
      noise, including the "not enough data" cases.
- [x] The recommendation rule is tested for the case it exists for: effort
      improving while load and reps stand still, and a lighter bar that fails
      harder.
- [x] Every requirement of the web UI has a browser scenario named after it in
      `test/web/drive.py`, and a failure names the scenario.
- [x] `cargo test` runs them, and a failure names the assertion rather than a
      diff of a whole table.
- [x] A session written by the CLI is stored finished while the web's live
      session stays open, and the effort levels' ranks are asserted, since the
      recommendation depends on their order.

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
