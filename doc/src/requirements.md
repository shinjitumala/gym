# Requirements for the web UI

This page is the spec for `gym web`, written as the scenarios a day goes
through. Each section is one scenario; the criteria under it are what
`python3 test/web/drive.py` asserts, and each scenario in that file is named
after the behaviour it checks, so a failure names the behaviour rather than a
page.

The rules that decide *which* number or *which* row to show are stated here
rather than left to the implementation, because they are what the screens are
judged against.

## Four definitions the screens depend on

### Sorted by recency

"Sorted by recency" appears on four lists, and each has its own clock:

| List | Ordered by | Ties |
|------|------------|------|
| Places | the date of the last session at that place | name |
| Exercises | the date of the last set of that exercise | name |
| Foods | the date of the last meal of that food | name |
| Presets | a meal preset's `last_used`; a gym preset has no clock | name |

A row that has never been used sorts last, and the list says so ("never used",
"never trained", "never eaten"), because "at the bottom" and "yesterday" are not
the same information when the list is what you pick from.

The order is computed on the server, in the query. The search box above a list
only narrows it; it never re-sorts, so what the reader sees is the order the
database chose.

### What the search box matches

Every box that narrows a list — the food, place, exercise and analysis pickers,
and the ones that build a preset — uses one rule, `narrows` in
`src/web/ui.rs`:

- The query is split on whitespace, and every word must be found in the row's
  text, in any order. A row's text is its name and, for an exercise, its muscle
  group, which is what makes `press chest` find `bench press · chest` — a plain
  substring test finds nothing, because no row says those two words in that
  order.
- A word is found when its letters appear as a run, or, failing that, in order
  with letters missing: `bnch` finds `bench press`, `sqt` finds `squat`, and
  `chikn` finds `chicken`. A two-word query still needs both words.
- Case is ignored.

Each word has to be found in one row's own text, so a query narrows a list of
rows; it does not search across rows. What it changes is which rows match, not
which order they come in: the ordering above is still the query's.

### Effort is compared before weight

A set carries four numbers: load, reps, effort and its time. Progress is not
"more load": a set taken closer to failure at the same load and reps is
progress, and the effort levels are defined so that moving *down* the list is
the improvement — `hard` is 3 or more reps from failure, `form fail` is 1 or 2,
`fail` is none.

So the recommendation for the next set reads the last session's best set against
the one before it in this order:

1. A **lighter bar is a regression**, whatever the effort — the one rule that
   outranks the others.
2. **Harder effort at the same weight and reps is progress.**
3. More **load** is progress, then more **reps**.
4. Easier effort at the same numbers is a regression; otherwise the set is
   repeated.

With progress and the phase's rep target reached, the advice is the next load
step (`+2.5 kg`) back at the target reps; below the target it is one more rep at
the same load. A regression drops `2.5 kg`. The first session of an exercise has
nothing to compare, so it sets the baseline instead of guessing.

`src/train.rs` is that rule, with its own tests; the exercise window only prints
the sentence it returns.

### A warmup is recorded, and is not a rung

A warmup set is stored like any other set — load, reps, effort, time — with
`warmup` as its effort. That value is part of the vocabulary the `effort` table
holds, and it is deliberately not on the ladder above: a warmup is not taken
near failure, so there is no rung it could stand on. Counted as the easiest
effort it would read as a regression and lighten the bar; counted as the hardest
it would read as progress and add weight. Either way the advice would be about
the preparation instead of the work.

So the marker is left out of everything that compares or counts sets, and the
four rules hold among the working sets alone:

- **The numbers to beat** are the best working set of the last session that had
  one, and of the session before it. A session that was only a warmup is not a
  session to compare against — the window reaches back past it — so an
  unfinished workout is neither a regression nor a baseline.
- **The PR badge and the analysis count work only.** The strength line is the
  best working set per session, sets per week is working sets per week, and the
  exercise list says how many sessions of the exercise did some, so a warmup
  neither makes a session nor moves a bar.
- **The row does not repeat a warmup.** After one, the pre-filled load, reps and
  effort still point at the working numbers, because the next set is the work
  the warmup prepared for.

A set is recognised as a warmup by its effort's name, not by its rank: the rank
is what the ladder reads, and a fourth number sorted among the three rungs would
be a lie by ordering. The seeded rank puts it after the rungs so a reader who
misses it never sees it ranked as a hard set.

## Daily · gym

The user starts a session, picks a place, builds the session's list — from a
preset or an exercise at a time — then presses `start sets` and records the sets
of that exercise, moving to the next one until the session is ended.

Acceptance criteria:

- [x] **A session starts from a place** picked from a searchable list sorted by
      the last session at each place, and the start's date and time are shown,
      with `now` filling both from the server's clock.
- [x] **Only one session is open at a time.** Starting another is refused with
      that sentence rather than silently opening a second.
- [x] **A preset fills the checklist** with its exercises in the preset's own
      order, and a preset with no exercises is refused rather than applied
      silently.
- [x] **The list shows progress** — what is not started, and how many sets each
      exercise has — and any exercise on it opens the exercise window.
- [x] **Picking an exercise adds it to the list and does not start recording.**
      `start sets` opens the window on the first exercise still to do, so the
      list can be assembled first and recorded afterwards.
- [x] **An open session is the whole screen**, and `end session` is its exit:
      the start form and the phase switch are not on the page behind it, so a
      mis-click cannot start a second session or lose the open one, and the
      session is still open until it is ended.
- [x] **A saved set is corrected in place** — `edit` on the row folds out its
      load, reps, effort and note, and saving writes that row alone, so a typo
      does not mean ending the session.
- [x] **The window names the next exercise and lets it be changed**: the default
      is the next one on the list, wrapping round, so a round is worked in the
      list's order; the dropdown opens any exercise on the list instead.
- [x] **The exercise window opens on the numbers to beat**: this session's sets
      with the time each was saved, the advice from
      [effort before weight](#effort-is-compared-before-weight), and the last
      three sessions of that exercise.
- [x] **The input opens on the numbers just done**: this session's last working
      set supplies the load, the reps and the effort; with none of this
      session's own sets to copy, the best set of the last session does; with
      neither, the load is blank and the reps are the phase's target. The effort
      dropdown offers `fail`, `form fail`, `hard` and the `warmup` marker, each
      with what it means in reps from failure.
- [x] **Saving a row stamps the time**, appends it to the exercise's sets, and
      keeps any other sets of that session.
- [x] **A saved set keeps the window.** The set joins the table of this
      session's sets in place and the bar's total follows it; the page never
      drops back to its loading line, and the exercise window is not rebuilt, so
      the numbers the reader is on stay where they are. The checklist behind the
      window is re-read when the window closes, where its counts are next read.
- [x] **X returns to the checklist**, where the exercise now reads as done.
- [x] **End session closes it.** A session with no sets is discarded rather than
      stored as an empty one, and either way the daily page goes back to
      offering a new session. The daily page lists no recent sessions: the log
      is read on the edit page and in the analysis, so the daily page holds the
      day.
- [x] **The cut/bulk switch is on this page**, and starting a phase records it
      from that date; past sessions keep the phase they were trained in.
- [x] **A session can be marked as a deload after it starts**, on the session
      row, so entering one is a choice and nothing switches by itself; the
      reminder stays visible whether or not it is taken up.
- [x] **A deload session's advice is half the last load and half the sets**,
      drawn from the last session trained normally and naming its numbers, so
      the reader can see what the halving is half of. The deload's own rows are
      shown in the history but are never the numbers to beat afterwards: the
      session after it compares against the normal session again.
- [x] **An exercise the log does not have is added to the list by name.**
      `add new` beside the picker takes a name, creates the exercise when the
      log has none of that name, and puts it on the list; a name the log already
      has is reused, so the list never points at two rows of one name. The new
      exercise has no muscle group until the edit page gives it one.
- [x] **A round of exercises is the list loop, not a recorded group.** Recording
      is: open an exercise from the list, save its sets, go back to the list and
      open the next one, in whatever order the session allows; each set is
      written on its own exercise at its own load, and the list ticks off as it
      goes. Sets recorded as one superset before the affordance was dropped keep
      their group and stay readable in the log; nothing writes that column now.
- [x] **The recent records are shown in full**: the last three sessions of the
      exercise are a table each, with the time every set was saved, its load ×
      reps, its effort and its note. A warmup is therefore read as the warmup it
      was, beside the work it prepared for, rather than as an anonymous pair of
      numbers among the others.

## Daily · food

The user starts a meal, times it, adds rows from a searchable food list, sees the
result before saving it, and sees the day against maintenance.

Acceptance criteria:

- [x] **The meal's time defaults to now**, and `now` fills the date and time
      from the server's clock.
- [x] **A preset loads its foods into the meal being entered** rather than
      recording them: the rows land in the draft, can be changed or removed, and
      nothing is written before `save`. Loading one still moves it to the top of
      the preset list, because that list is ordered by what the reader reaches
      for.
- [x] **A row is added from a food list** sorted by the last time that food was
      eaten, searchable by name.
- [x] **A row previews its own calories and macros** while its amount is typed,
      and the meal previews the total of its rows.
- [x] **A row can be removed before saving**, and nothing is written until
      `save`: a half-entered meal is never stored.
- [x] **The day's totals** — calories, protein, fat and carbohydrate — are at the
      top, with the difference against the estimated maintenance when that
      estimate is available, and what is still missing when it is not.
- [x] **The saved rows appear in the day's table immediately**, with their time,
      amount and macros.
- [x] **A save does not blank the page.** Saving a meal leaves the form, the
      day's totals and the table where they were while the page re-reads, so the
      row just written appears in place and nothing the reader was looking at
      disappears and comes back.
- [x] **The form projects the day**: under the draft, the same four figures the
      top of the page shows — calories, protein, fat and carbohydrate — are
      shown with the draft added, so the meal is judged before it is saved.

## Daily · weight

Acceptance criteria:

- [x] **A weigh-in is a date, a time defaulting to now, kilograms, optional body
      fat and an optional note**, and body fat left blank is stored as unknown,
      not as zero.
- [x] **The last weigh-in and the trend** are shown with the phase's goal or a
      statement of what data is missing.
- [x] **The recent weigh-ins are listed newest first** with their body fat.

## The look

Acceptance criteria:

- [x] **The scrollbar is styled**: thin, a green-900 thumb on a black track, from
      Tailwind v4.3's `scrollbar-*` utilities, and the scrolling column reserves
      its room so a page that grows a bar does not shift sideways.
- [x] **Every chart has its axes**: dates along the bottom and round numbers up
      the side, with the deload bands and the phase colours still readable
      behind them.

## Edit

Edit is reached from the bottom of each mode's daily page, and manages that
mode's data. The tabs are only **gym**, **food** and **weight**: there is no
edit as a tab of its own, and no analysis page for food — the intake chart is
read against the weight trend, so it lives on the weight analysis page.

Acceptance criteria:

- [x] **Gym · edit** manages sessions and sets (load, reps, effort, note), the
      deload flag, exercises with their muscle groups and video link, gym
      presets (a named list of exercises), places and muscle groups.
- [x] **A gym preset is built from a searchable exercise list**: a filter box
      over the exercises, searchable by name or muscle group, a click for each
      one that belongs in the preset, and `remove` on a row that does not. A new
      preset starts with no rows and an existing one opens with its own, in its
      own order, and saving rewrites the list in one transaction; a finished
      session keeps the exercises it recorded.
- [x] **Food · edit** manages meals, foods and meal presets.
- [x] **A meal preset is built from the same searchable food list**: a click
      per food, an amount beside each row it adds, and the preset keeps the
      order they were picked in.
- [x] **Weight · edit** manages past weigh-ins.
- [x] **A row is edited in place**, and a delete that the log still depends on
      is refused with the count of rows that use it.
- [x] **Every edit affects only the row it names**, and deleting a set closes the
      gap in its session's order.

## Analyse

Acceptance criteria:

- [x] **The chart's labels are readable**: the dates and numbers are painted in a
      colour other than the background, and the numbers sit in the chart's margin
      rather than over the plot.
- [x] **Gym · analyse** plots one chosen exercise's estimated 1RM, overlaid with
      sets per week for that exercise and for the exercises that share its main
      muscle group on a second scale, and the time range can be changed.
- [x] **The exercise list is searchable**: a filter over the list that has a
      trend, narrowing by name or muscle group, and picking one draws it. The
      list, not a select, is what makes a log of two hundred exercises usable.
- [x] **Gym · analyse shades the deload weeks** behind the strength line rather
      than dropping them, so the trend keeps every session's point, and the
      legend names the band.
- [x] **Weight · analyse** plots weight and body fat over a chosen range, with
      the weight line coloured by the phase each weigh-in was taken in, and the
      deload weeks shaded behind it.
- [x] **The range opens on the data**: a year by default, and when the window
      still holds fewer than two weigh-ins the page says how many it holds and
      when the last weigh-in was, so an empty chart is explained rather than
      mistaken for a missing log.
- [x] **The weight line is the range's weigh-ins**, not the slope's trailing
      window: a log whose latest weigh-in is older than the window still draws a
      line, and only the slope is withheld.
- [x] **Under the weight line, the intake bars** are drawn per day, switchable
      between calories and each macro, and pressing or hovering a bar names the
      foods that made that day.
- [x] **Under the bars, the foods in the range are ranked** by what they
      contributed to the metric on show — calories or one of the macros — with
      each food's share of the range beside it. The ranking covers the whole
      chosen range, not the picked day.
- [x] **The analysis pages write nothing**: every request from them is a read.
- [x] **The food ranking has no page**: the range's foods are ranked under the
      bars, and `/food/analyse` is not a route.

### The per-food ranking has no page of its own

The redesign retired the food analysis page, and the ranking moved with the
intake chart instead of returning as a page: the chart is read against the
weight trend over one window, and a ranking is the same window summed by food,
so putting it under the bars answers "which food is supplying my calories"
beside the day-by-day question without a second query or a fourth analysis
surface. Food still has no analysis page, and the food list on food's edit page
stays ordered by recency, because that list is a picker's order and not a
ranking. A ranking that needed its own window would be a second analysis page
for food, and that is the thing this design refuses.

## How it is tested

`test/web/drive.py` builds the site, seeds a scratch database from
`test/web/fixtures.sql`, starts `gym web` on a free loopback port and drives a
headless browser over the Chrome DevTools Protocol — no Playwright, no Node, no
extra dependency. It writes its screenshots and its database under
`$FPR_SCRATCH_DIR` (default: a fresh temporary directory), never into the
repository.

```bash
$ python3 test/web/drive.py
$ GYM_BIN=target/release/gym python3 test/web/drive.py   # skip the build
```

The recommendation rule has unit tests of its own, so the case that matters most
— effort improving while load and reps stand still — is asserted without a
browser:

```bash
$ cargo test train::
```
