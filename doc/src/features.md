# Features and acceptance criteria

This page is the working list of what `gym` should do. Each entry states what
is true today, then the criteria that decide the entry is done. The criteria
are the editable part: rewrite them to describe the behaviour that is wanted,
and tick a box only when the behaviour is there and has been seen working.

To have a feature worked on, edit its criteria and name its heading to the
agent. Keep the `## Feature:` heading, the `Status:` line and the checkbox
list, so a reader can tell what is claimed from what is verified.

Status values: **works** (implemented and exercised), **partial** (some of it
exists), **missing** (nothing yet).

## Feature: Log a training session

Status: works. `gym new` asks for place, time, description, then exercises and
their sets, printing the last three sessions of history for each exercise.

Acceptance criteria:

- [ ] A session is written to `session/YYYY-MM.toml` with its place, date and
      the ids of its sets.
- [ ] Each set is written to `set/YYYY-MM-DD.toml` immediately, with load,
      reps and notes.
- [ ] Before the first set of an exercise, the last three sessions of that
      exercise at that place are printed as `date:` plus `load x rep` rows.
- [ ] An unknown exercise name is created in `exercise.toml` instead of being
      refused.
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
- [ ] Each meal is written to `meal/YYYY-MM-DD.toml` with food id, amount and
      description.
- [ ] The amount is a multiplier of the food's per-unit numbers.
- [ ] A food may be registered during the flow, with protein, fat and
      carbohydrate optionally unknown.
- [ ] Registering a name that already exists in `food.toml` is refused with a
      message saying so.
- [ ] `Add more food?` defaults to no, ending the command with the updated
      totals.

## Feature: Log weigh-ins

Status: works. `gym weight` asks for date, kilograms, body fat percentage and a
note.

Acceptance criteria:

- [ ] The weigh-in is written to `weight/YYYY-MM.toml`.
- [ ] Body fat may be left unknown without aborting the command, and an
      unknown value is distinguishable from a measured zero.
- [ ] The note may be empty.
- [ ] Two weigh-ins on the same day are both kept, in the order they were
      entered.

## Feature: A data directory a human owns

Status: works. The data directory is the database: TOML files, one row per
`[e.N]` table, no server and no account.

Acceptance criteria:

- [ ] Every command reads and writes only inside `db` and `repo`.
- [ ] Rows the tool did not create in this run are never rewritten or
      reordered.
- [ ] No command deletes a row.
- [ ] The layout and each schema are documented in
      [the data files](data-files.md), and the document matches the files a
      command actually writes.
- [ ] A hand edit that keeps ids intact is picked up by the next command.

## Feature: Bootstrap a fresh data directory

Status: missing. A new data directory must be assembled by hand: the four
index files need at least an empty `[e]` table, or the first save aborts with
`Failed to write to ''`.

Acceptance criteria:

- [ ] Pointing the config at an empty directory and running any command either
      creates the missing files and directories itself, or refuses with a
      message naming every path it needs.
- [ ] No command can fail with an empty path in its error message.
- [ ] A freshly created directory accepts `gym new`, `gym food` and
      `gym weight` end to end, with no hand-written files.

## Feature: Sync the data directory to a remote

Status: works. `gym sync` pulls, commits changes as `upd.`, and pushes to every
configured remote.

Acceptance criteria:

- [ ] With a clean tree, `gym sync` prints `There is nothing to commit.` and
      pushes.
- [ ] With changes, everything under `repo` is committed in one commit.
- [ ] A failing pull is reported as a failure, and the command's exit status
      reflects it.
- [ ] The commit message says what changed (which part of the log), not `upd.`

## Feature: Read the log back

Status: partial. Only two views exist: the food day's totals, and the last
three sessions of an exercise during `gym new`.

Acceptance criteria:

- [ ] A command prints the history of one exercise over a chosen period:
      date, load, reps, and the best set per session.
- [ ] A command prints the week's or month's training volume per muscle group.
- [ ] A command prints the body weight and body fat trend, one row per
      weigh-in in the period.
- [ ] A one-rep-max estimate is reported per session, using the existing
      rep-factor table.
- [ ] Every view reads only the files and rows it needs, and prints nothing
      else.

## Feature: Correct a logged row

Status: missing. Rows can be added but not edited or removed by the tool.

Acceptance criteria:

- [ ] A command lists recent rows of one kind (set, meal, weigh-in) and lets
      one be picked.
- [ ] The picked row can be edited field by field, or deleted, without
      renumbering any id another row refers to.
- [ ] A deleted set is removed from its session's `sets` list.
- [ ] A deleted meal is reflected in the day's totals.
- [ ] The file the row lives in is the only file rewritten.

## Feature: Exercise and food metadata

Status: partial. Exercises can be created with a name only; muscle groups and
food numbers have no editing path once written.

Acceptance criteria:

- [ ] An exercise's muscle groups can be assigned and changed from prompts.
- [ ] A food's calories, protein, fat, carbohydrate and description can be
      changed after it was registered.
- [ ] A food or exercise that a row still refers to cannot be deleted.
- [ ] Muscle-group names are managed in one place and referenced by id.

## Feature: Non-interactive input

Status: missing. Every command prompts; nothing can be scripted.

Acceptance criteria:

- [ ] Each logging command accepts its answers as flags or on stdin, and
      writes the same rows the prompts would.
- [ ] The default remains interactive: no flags means prompts.
- [ ] A malformed value exits non-zero with a message naming the field and the
      value, and writes nothing.
- [ ] One worked example per command appears in the documentation, scripted.

## Feature: Web view

Status: missing. `gym web` parses an address and then panics with
`not yet implemented`.

Acceptance criteria:

- [ ] `gym web` serves the log at the given address (default
      `0.0.0.0:8080`) without touching the data files.
- [ ] The pages show the day's food totals, the weight trend, and per-exercise
      progression.
- [ ] The server reads the data directory once per request, so a sync from
      another terminal is visible without restarting it.
- [ ] A read-only server cannot be used to change the log.

## Feature: Build and install

Status: works. `./install.sh` formats, builds a release binary and installs it
into `$CARGO_HOME/bin` atomically; `./deploy.sh` cross-compiles for
`aarch64-linux-android`.

Acceptance criteria:

- [ ] `./install.sh` is idempotent and safe while `gym` is running.
- [ ] The installed binary reports which build it is (`gym --version`).
- [ ] `./deploy.sh` states its NDK prerequisites and fails with a message, not
      a linker error, when they are missing.
- [ ] The documentation build (`mdbook build doc`) is part of the install or
      checked separately, and a broken link fails it.

# Overhaul plan
Migrate back to sqlite databases. Why? AI had made the cost of CRUD to near zero. The need tonhave human-readable and editable databases has disappeared.

Keep CLI interface as is, add a web UI that runs locally by `gym serve` (leptos)

Theme similar to ~/g/fpr-rp chat3.

Ad hoc migration scripts stay in ~/g/gym_data (current real data)

## UI design
Have a tab for daily use (most used path for daily use), intervrntion (editing, fixing stuff), and analysis (looking back at progress over long term, trends, for show and analysis) fpr the food and gym (training record) respectively.

Organize them into tabs with six tabs total.

### Food
#### Daily use
Record what and how much you ate when (same purpose as gym food today).
Record weight and bodyfat (same purpose as gym weight today).
##### New features
Use presets for a set meal for repetitive ones. Sort by last used. Define in the intervension tab.
Show weight trend and show if you're meeting goals depending on weather your bulking or cutting. Bulking is generally +1kg / month trend, And cutting is just going below maintenance. Calculate statistically the weight trend accpunting for slight errors in the weight measurer. Estimate maintenance calories and show diff. Identify possibly incorrectly valued food based on recorded calories vs actual weight change. Take into account poasible water weigjt change when switching between vits and bulks.

#### Intervension
Edit past records.
Edit food items to change calorie and macro value.

#### Analysis
Show graph of weight and bf trend overlayed above a bar graph of calorie intake and macros that show which food they came from.
Favorite food ranking, etc.

### Gym
#### Daily use
Record training as we do today. (gym new)
Add, edit places as we do today.

##### New feature
Have a preset for recording effort along the optional notes (see ~/g/gym_data for examples)
Indicate when you get a PR.
Switch between bulk and cut mode (change to target weight and reps)
Add deload reminders and target change to half weight and sets during deload.
Support super sets natively. (see current data to indersfand how we're working arpumd it currently and migrate data.)
Use lists (set of preset exercises for a session as an ordered checklist. Can perform out of order since a splot may be busy.) Lists are defined in the ontervention tab.

#### Intervention
Edit past records, body part set count per exercise, exercise name, etc.

##### New feature
Allow linking a YouTube video to a specific exercise

#### Analysis
Trend of strength per body part by estimated one rep max.
Trend of sets per muscle group.
