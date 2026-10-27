# The web UI

`gym web` serves the same log in a browser, on loopback. The first paint is
rendered on the server and the page then hydrates, so it works before the WASM
arrives and stays interactive afterwards.

```bash
$ gym web
gym web on http://127.0.0.1:8080
```

```bash
$ gym web --addr 127.0.0.1:9000   # another port
```

The default is `127.0.0.1:8080`: loopback, so nothing on the network can reach
it. Passing another address makes it reachable from wherever that address is —
there is no authentication, so only do that on a network you trust.

The behaviour the pages are judged against is on
[Requirements for the web UI](requirements.md), one scenario per section.

## Three tabs, and the mode's other pages at the bottom

The log records three things, so the bar has three tabs: **gym**, **food** and
**weight**. A mode's **edit** and **analyse** pages are not tabs. They are
buttons at the bottom of that mode's daily page, so the bar never grows and a
phone never gets a menu:

| Tab | Daily | Edit (bottom) | Analyse (bottom) |
|-----|-------|---------------|------------------|
| gym | The phase switch and the start form, or — once a session is open — the session itself, full screen. | Sessions and sets, exercises and muscle groups, gym presets, splits, places. | One exercise's strength, overlaid with sets per week for it and its muscle group, and the deload weeks shaded. |
| food | Today's totals against maintenance, the meal being entered with its projected day, the day's meals. | Meals, foods, meal presets. | — the intake chart lives under weight, beside the trend it is read against. |
| weight | The trend and the last weigh-in, the weigh-in form, recent weigh-ins. | Past weigh-ins. | Weight and body fat by phase, with the intake bars underneath and the range's foods ranked beside them. |

`/status` shows the database file, the remote, and the row counts.

A page with more than one subject puts them behind a **segmented switch** rather
than a route, so the three routes stay three.

Each page is one query against the server, so it shows what the database held
when it was asked. A form that saves bumps a counter and the page re-runs its
query, so the row you just wrote is there — and the one-line reply the save
returns is gone with the re-render, which is why the test waits for the row and
not for the message.

## Recording

- **A session is open until it is ended.** Starting one writes a session row;
  the sets of each exercise are appended to it as they are saved, each stamped
  with the minute it was saved. `end session` closes it, and a session with no
  sets is discarded instead of stored as an empty one.
- **An open session takes the whole screen.** The phase switch and the start
  form are behind it rather than beside it, so a session has one exit — `end
  session`, in the row that never scrolls away — and nothing else on the page
  can be clicked by mistake. The screen is not a route: leaving the page and
  coming back finds the same open session, because what makes a session open is
  its row, not the view. `✕` inside the exercise window goes back to the
  session's list; it does not end the session.
- **The list comes first, then `start sets`.** Applying a gym preset puts its
  exercises on the list in the preset's order, and picking an exercise from the
  picker adds it; picking one does *not* open its window. `start sets` opens the
  window on the first exercise still to do. ☐ and ☑ say what is left.
- **The list counts the work against the preset's target.** A preset gives each
  of its exercises a number of working sets, and the row reads `2/3 set(s)` — ☑
  once the target is met — so a session says how far into an exercise it is.
  Warmup sets are recorded and shown and are not counted, so a warmup neither
  starts an exercise nor finishes one; an exercise picked on its own has no
  target, so its row counts its sets without a `/n`.
- **A split offers the day's preset; it does not fill the list.** A split is the
  week the training is scheduled by: which preset each weekday asks for, with
  one split in use. A session started on a day it schedules shows the offer above
  the list — the weekday, the split and the preset — and `apply push` fills the
  list exactly as the preset's chip would. Until then the list is empty, because
  the week is a suggestion and the list is the reader's plan for the hour; the
  offer disappears once the list has anything on it, and with no split in use it
  never appears.
- **An exercise the log has never had is added by name.** `add new`, beside the
  picker, takes a name and puts that exercise on the list, creating the row in
  the log if no exercise of that name exists yet. It arrives with no muscle
  group and no video link, which the edit page fills in; a name the log already
  has is reused rather than created twice.
- **A session is the list, one exercise at a time.** Recording is a loop: open
  an exercise from the list, save its sets, go back to the list — `✕` or Esc —
  and open the next one. A round of exercises done back to back is that loop,
  not a stored group: each set stands on its own exercise at its own load, and
  the list is ticked off as it goes.
- **The exercise window is where a set is recorded.** Its row opens on the
  numbers just done: the last working set of this session for that exercise, or
  the best set of its last session when this session has none. The effort
  dropdown offers the three rungs with what each means, and the advice for the
  next set is printed beside the last three sessions of that exercise.
- **The recent sessions are shown in full.** The last three days of that
  exercise are one table each: the time each set was saved, its load × reps,
  its effort and its note. Nothing is folded into a `load × reps` list, so a
  warmup is read as the warmup it was — the named effort beside the work it
  prepared for — and a note taken on a set is not lost between sessions.
- **The window says which exercise is next, and lets it be changed.** By default
  that is the next one on the list, wrapping round, so the list's order is the
  order a round is worked in; the dropdown beside it opens the window on any
  exercise on the list instead. The sets keep the session-wide `position` they
  were saved in, so the order they were done in is recorded, not just shown.
- **A saved set is corrected where it is.** A typo is found the moment the row
  is saved, so the row folds out in the session's table with its load, reps,
  effort and note; saving writes that row only. Nothing has to be ended and
  re-entered to fix a number.
- **Old superset groups stay readable.** Sets recorded as one superset before
  the affordance was dropped keep their `superset` column and the `ss` mark on
  the edit page; nothing writes that column now, so a group is history rather
  than something the session list offers.
- **A deload is marked on the session**, never entered automatically: when the
  configured number of weeks has passed since the last one, the daily page says
  so and leaves the decision to the reader. A deload session's advice is half
  the last load and half its sets, named from the last session trained
  normally. The deload's rows stay in the history, marked, but the sessions
  after it compare against the normal numbers again — a planned dip is not a
  regression.
- **Effort is part of the record, not a note.** A set's effort is a row in
  `effort`, so its rank (closer to failure is a smaller number) is what the
  recommendation compares before load and reps. `warmup` is a value of that
  vocabulary and not a rung of the ladder: a warmup is logged with it and then
  left out of the advice, the pre-filled numbers, the PR badge and the analysis.
  See [effort before weight](requirements.md#effort-is-compared-before-weight).
- **A meal is a draft until it is saved.** Rows are added from a food list and
  previewed as they are typed; `save` writes them in one transaction. A preset
  loads its foods and amounts into the draft, and nothing is written by that: the
  reader edits the rows it filled, then saves. Loading a preset stamps it as
  used, so the list keeps sorting by the preset reached for first.
- **The draft says what the day would come to.** Under the rows, the projected
  day repeats the four figures at the top of the page — calories, protein, fat
  and carbohydrate — with the draft added, so the meal is judged before it is
  saved rather than after.
- **A weigh-in is separate from a meal**, and body fat left blank is stored as
  `NULL`, which is not the same as a measured `0`.
- **The phase switch is on the gym daily page.** Starting a phase ends the old
  one from that date; past sessions keep the phase they were trained in, which is
  what colours the weight analysis.

## Editing

Every editing row opens in place: **edit** folds out the fields, **save** writes
them, **delete** removes the row. Deleting a set closes the gap in its session's
order. A food or exercise still referenced by a row cannot be silently deleted:
the refusal says how many rows use it.

- **Merging an exercise** moves every set and list item to the survivor, unions
  the muscle groups, and deletes the other. There is no undo, so the button
  says so.
- **A gym preset is edited in place.** Its row folds out into a name and a list
  of exercises: the filter box above the candidates narrows them by name or
  muscle group, a click appends a row, `remove` drops one, and the number beside
  a row is the working sets that exercise is there to do (three on a fresh pick,
  and at least one). Saving rewrites the preset in one transaction. A new preset
  starts empty and grows a row per pick. Finished sessions store the exercises
  they recorded and the targets they were started against, not the preset, so
  rewriting one never rewrites history.
- **A split is edited as a week.** Its row folds out into a name, an `in use` box
  and seven rows — one per weekday, each a dropdown of the presets, with `rest`
  for a day that asks for nothing. Saving rewrites the week in one transaction.
  `in use` is the whole switch: marking one split clears the mark on the other,
  and clearing the mark turns the offers off without deleting the schedule. A
  split needs at least one day with a preset; a preset a split schedules cannot
  be deleted until the split stops naming it, and the refusal says how many days
  still do.
- **A meal preset is built the same way.** The filter box narrows the food list,
  each click adds a row, and the amount beside it is typed on the row. The
  preset keeps the order the foods were picked in, which is the order the daily
  page loads them in.
- **A deload is a session's flag**, not a set's: the toggle sits on the session
  row, because a deload is a week's plan.

## Charts

Both analysis pages draw with inline SVG, and both say what their range is. Every
chart labels its axes: four dates along the bottom, `YYYY-MM-DD`, spread over the
window, and round numbers up the side — whole kilograms, whole calories, sets per
week to one decimal — with a dashed gridline at each. The numbers are picked at a
1/2/5 step, so an axis reads `80, 82, 84` rather than `80.37`, and the date labels
are the days the data actually spans, not the range the reader asked for.

An SVG `text` fills **black** unless it is told otherwise, and this page's
background is black: the colour class on the `<svg>` colours the element, not the
glyphs. Every axis label therefore carries an explicit `fill`, and a scenario in
`test/web/drive.py` asserts the computed fill is not the background.

**The line is the range, not the slope's window.** The weight chart draws every
weigh-in in the chosen range; the trailing window is what the *slope* needs, and
`trend.points` is only that window. A log whose last weigh-in is older than the
window used to draw body fat — built from the whole range — and a blank weight
line beside it, saying "0 weigh-ins in the window". The line now comes from the
range's own observations, so that page cannot contradict itself.

A deload week is a band behind the line rather than a gap in it: the session
still has its point and the band says why the load dipped. The weight line's
colour comes from the phase each weigh-in was taken in, so a phase boundary is
visible without a second mark.

**No chart library.** `leptos-chartistry` was evaluated and not adopted: it would
put a chart crate and its `leptos-use`/`web-sys` dependencies into both halves of
the render path, to draw a polyline, a stack of rectangles and a handful of
labels — which are already written here in about four hundred lines, in the same
look as the rest of the app, with the threshold rules (no line from two
weigh-ins) expressed in the same place as the drawing. The four charts do not
justify a dependency; the axes above were the whole of what was missing.

## Where body weight is read

Weight and body fat are recorded on **weight · daily** and charted on
**weight · analyse**; food keeps no weight of its own, and its analysis page was
retired with the redesign. So "the weight is missing" is a question about the
analysis window, and the page answers it two ways:

- The range opens on **365 days**, not a quarter, because body weight is read
  over months and a log with a gap of a few weeks would otherwise open on an
  empty chart. The line is drawn from the range's weigh-ins, whatever the
  slope's own trailing window holds.
- When the window still holds fewer than two weigh-ins, the page says how many
  it has and when the last one was, so a narrow range reads as a narrow range
  rather than as a log with no weight in it. The same sentence covers body fat,
  which is drawn only when at least two weigh-ins in the window measured it.

```text
0 weigh-in(s) in the last 90 days, and a line needs two. The last weigh-in is
dated 2026-04-14: widen the range to reach it, or log a new one.
```

The intake bars live on the same page for the same reason: a window with no
meals in it is a fact about the window, and the statement above it says so.

Under the weight line, the intake bars show one day at a time. Beneath them, the
foods in the whole range are ranked by the metric the bars are showing, with each
one's share — the same window summed by food, which is what the per-food view is
now. Food has no analysis page; the reason the ranking lives here is written down
on [the requirements page](requirements.md#the-per-food-ranking-has-no-page-of-its-own).

## What the numbers mean

The statistics are the part with rules, and they are on
[the overhaul](overhaul.md#weight-and-intake-statistics) page. In short:

- A slope needs enough weigh-ins in the window, and the page says how many are
  missing rather than drawing a line from two mornings.
- A goal is met only when the trend's whole uncertainty band is inside the goal
  range; a band that straddles the boundary says "too noisy to say".
- Maintenance is withheld until enough days were actually logged.
- Days, never foods, are flagged when intake and weight disagree.

## Building it

The look is Tailwind v4: one `@import "tailwindcss"` in `style/main.scss` and
classes on the markup, with no theme file. The base look (black, a green ramp,
blocky controls, 2px borders, no rounded corners) is one `APP_CLASS` string on
the app root, whose descendant selectors set the defaults; markup then says only
what is different about a control.

The scrollbar is part of that look: `scrollbar-thin scrollbar-thumb-green-900
scrollbar-track-black` on the app root — a thin bar, a green-900 thumb on black —
and `scrollbar-gutter-stable` on the scrolling column, so a page that grows a
scrollbar does not shift its controls sideways. Those four utilities are
first-party from **Tailwind v4.3**, which no class in the markup can ask for: the
release is whatever `LEPTOS_TAILWIND_VERSION` names, and without it cargo-leptos
uses the one it was built against. So every path that reassembles the stylesheet
pins the same release — `./install.sh`, `./deploy.sh` and `test/web/drive.py` all
export `LEPTOS_TAILWIND_VERSION=v4.3.3` — and a new one has to as well.

What that buys is measured, not assumed. Built without the pin (cargo-leptos
0.3.9 fetches v4.2.1) the stylesheet holds no `scrollbar` rule at all: the four
classes generate nothing and the browser falls back to its own bar. The same
source under v4.3.3 emits `.scrollbar-thin{scrollbar-width:thin}`,
`.scrollbar-thumb-green-900`, `.scrollbar-track-black` and
`.scrollbar-gutter-stable`. The stylesheet's first line names the release that
made it, so a build can be checked rather than trusted:

```bash
$ head -1 target/site/pkg/gym.css
/*! tailwindcss v4.3.3 | MIT License | https://tailwindcss.com */
```

The hydrated bundle is **inside the binary**, so installing stays one file:

```bash
$ LEPTOS_TAILWIND_VERSION=v4.3.3 cargo leptos build --release   # the stylesheet and the WASM half, into target/site/pkg
$ cargo build --release --features ssr                          # the binary, which embeds them
```

`./install.sh` runs both in that order. The second pass matters: `cargo leptos`
starts the binary's build before the site is complete, so that pass can embed
nothing, and recompiling the binary is what puts the assets in.

`./deploy.sh` makes the same two passes for Android: the bundle with
`cargo leptos build --release`, then `cargo build --target aarch64-linux-android
--release --features ssr` for the binary that embeds it. `cargo leptos` has no
option for the server's target — it builds the WASM half for
`wasm32-unknown-unknown` and the server for the host — so the Android binary is
cargo's own cross build, with `ssr` for the server and the NDK's clang as the
linker. The script then reads the built script and stylesheet back out of the
binary, so a build that embedded nothing fails at the deploy rather than on the
phone.

A build without the bundle — a plain `cargo build` — is still a working CLI;
`gym web` then refuses to start and says to rebuild, instead of serving an empty
page.

**Build in release to look at the site.** Leptos's debug build of the WASM emits
`<!>` comment markers inside every element to keep positions stable while it
renders, and the server's debug build does not; a debug bundle therefore
hydrates against a DOM that has no markers and panics inside `tachys` on the
first element. The release build, which `install.sh` makes, does not carry them.

## Why the forms read the DOM

A field's value is read from the element when the form is submitted, not from a
signal mirrored on every keystroke. The element is the source of truth: a mirror
can lag (autofill, IME composition, a test harness setting the value), and a
mount that re-renders a reactive `value` can overwrite what was typed. The
fields are therefore uncontrolled — an initial `value` and nothing reactive —
and the submit reads them.

The server functions take a JSON body (`#[server(input = Json)]`). The default
codec is URL-encoded, which cannot carry a list of rows, and a form here
regularly sends one — the meal rows, a preset's exercises, a set of muscle
groups.

A filter box is the one place a signal mirrors the DOM as it is typed, and it
does so only to narrow a list: the rows are the server's, the order is the
server's, and one predicate in `src/web/ui.rs` decides which of them stay —
see [what the search box matches](requirements.md#what-the-search-box-matches).
A preset form keeps the filter box mounted while its picks change, because a
box that was rebuilt on every pick would lose the query as soon as the reader
used it.

## A re-read keeps the view

Every page's data is one `Resource` against one server function, and the view it
renders sits in a `<Transition>`. The boundary is what the reader sees while the
next read is in flight: `Suspense` drops back to its fallback on *every* pending
read, so a save collapsed the page to `Loading…` and rebuilt it — the day's
table, its totals and the row being typed all disappeared and came back — while
a `Transition` keeps the view it has until the next one is ready, and its
fallback belongs to the first paint alone, when there is nothing to keep. The
fallback is one line inside the boundary, never the page shell: the header, the
tabs and the action row are drawn once by the app root and must not go with the
data.

A save is followed by that page's re-read: the write bumps a counter the page's
`Resource` is keyed on, and the re-read is what makes the row just written
appear. Three things keep the re-read from moving the ground under the reader.

- **The exercise window re-reads itself, not the page.** A set saved or
  corrected in the window bumps the window's own counter, so the table of this
  session's sets grows under the form. The page is re-read when the window
  closes, because the checklist's counts are what is read next, and the
  session's set total in the bar counts the rows added since the page was last
  read, so it is right while the page's copy is still the older one. The exit
  from the session is the page's own action for the same reason: a read landing
  while `end session` is in flight rebuilds the screen, and a result that lived
  in the screen would be dropped with it.
- **The set row is re-seeded from the numbers just done.** Keeping the window
  means keeping the same input elements, and a field the reader typed into
  ignores a `value` attribute written beside it — the attribute moves and the
  box does not. So the row's load, reps and effort are set as **properties**
  (`prop:value`), which is what lets a re-read put the next set's numbers in
  them: after a working set, that set; after a warmup, still the work it
  prepared for. Without that, the second save of a session repeated the warmup
  the first one had just marked.
- **A window belongs to one exercise.** Opening another exercise builds a new
  window, whose `Transition` has nothing to keep and so shows the fallback: the
  reader is never shown the previous exercise's sets under a form that would
  write to that exercise. A re-read of the same exercise is the case the
  `Transition` is for.
- **The control that starts a write is disabled while it runs**, from its own
  action's pending state — that control, not the page behind it: the list, the
  filter boxes and the other rows stay live, so a save never freezes the page it
  was made on. A disabled submit button is also what refuses a second `Enter` in
  a field, since the browser does not submit a form whose submit button is
  disabled, so the fields themselves stay live and nothing greys out. Where a
  second request would be the *same* write the control stays live too: a picker
  row adds an exercise with `ON CONFLICT DO NOTHING`, so two quick clicks write
  one row, and disabling the row would only refuse the second click.

## Caching

Every response carries `Cache-Control: no-cache`. The bundle's URLs are stable
across builds, so a browser that kept a script and paired it with a newly built
WASM would hydrate two different DOM shapes and panic inside `tachys`.
`no-cache` still allows a 304; it only forces the check, so half of a previous
build cannot survive an install.

## Testing it

`test/web/drive.py` drives the real site in a headless browser: it builds,
seeds a scratch database, starts `gym web` on a free port and walks the
scenarios on [the requirements page](requirements.md). It needs only Python's
standard library and one Chromium-based browser.

```bash
$ python3 test/web/drive.py
```

The fixture log in `test/web/fixtures.sql` is dated **relative to the run**,
never to a calendar day. A row names a day — `{{at:meal_newest:08:00:00}}` — and
the `DAYS_AGO` table in `drive.py` says how many days before today that name
sits; the script fills them in before seeding. The instant carries the machine's
offset for its own day, so the server reads back the same local date even when a
daylight saving change falls inside the log.

That is what keeps a scenario about recency honest. The food picker is ordered
by the last meal of each food and says "yesterday" beside the newest one, and
the weight chart's ends are the log's oldest and newest weigh-in; written as
absolute dates, the same fixture reads "2 days ago" the day after it was
written, and the suite fails on the calendar rather than on the app. Where a
scenario reads a date back, it asks the seeded database for the day it wrote
(`weigh_in_days()`), so the assertion is about the data and not about today.

Screenshots and the scratch database go under `$FPR_SCRATCH_DIR`, never into the
repository.

The search scenarios type what a substring test cannot match — `press chest` for
the two chest presses, `sqt` for the squat, `chikn` for the chicken — because a
query that already worked would pass whether the rule changed or not. The rule's
own cases are unit tests beside it in `src/web/ui.rs`.
