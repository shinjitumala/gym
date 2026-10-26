# Requirements for the web UI

This page is the spec for `gym web`, written as the scenarios a day goes
through. Each section is one scenario; the criteria under it are what
`python3 test/web/drive.py` asserts, and each scenario in that file is named
after the behaviour it checks, so a failure names the behaviour rather than a
page.

The rules that decide *which* number or *which* row to show are stated here
rather than left to the implementation, because they are what the screens are
judged against.

## Three definitions the screens depend on

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
      is the next one on the list, wrapping round, which is the alternation a
      superset needs; the dropdown opens any exercise on the list instead.
- [x] **The exercise window opens on the numbers to beat**: this session's sets
      with the time each was saved, the advice from
      [effort before weight](#effort-is-compared-before-weight), and the last
      three sessions of that exercise.
- [x] **The input defaults to the last weight used** for that exercise, and the
      effort dropdown defaults to the last effort used and offers exactly
      `fail`, `form fail` and `hard`, each with what it means in reps from
      failure.
- [x] **Saving a row stamps the time**, appends it to the exercise's sets, and
      keeps any other sets of that session.
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
- [x] **A superset records two or more exercises as one unit**: one load for the
      group, a rep count for each exercise, saved in one action. A blank rep row
      keeps that exercise out, so the group is exactly the exercises that were
      given reps, and the list marks each of them done.
- [x] **The order a superset was performed in is recorded**, not inferred: every
      set carries the session-wide position it was saved at, so two sets of one
      group at positions 3 and 4 say which exercise came first.

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
