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
| gym | The phase switch and the start form, or — once a session is open — the session itself, full screen. | Sessions and sets, exercises and muscle groups, gym presets, places. | One exercise's strength, overlaid with sets per week for it and its muscle group, and the deload weeks shaded. |
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
- **The exercise window is where a set is recorded.** It opens on the last load
  used for that exercise, offers the three efforts with what each means, and
  prints the advice for the next set beside the last three sessions of that
  exercise.
- **The window says which exercise is next, and lets it be changed.** By default
  that is the next one on the list, wrapping round — which is what makes a
  superset alternate between its exercises — and the dropdown beside it opens
  the window on any exercise on the list instead. The sets keep the session-wide
  `position` they were saved in, so the alternation is recorded, not just shown.
- **A saved set is corrected where it is.** A typo is found the moment the row
  is saved, so the row folds out in the session's table with its load, reps,
  effort and note; saving writes that row only. Nothing has to be ended and
  re-entered to fix a number.
- **A superset is one unit.** Two or more checklist exercises are given one
  load and a rep count each, and saved together: the sets share the group's id,
  so the log can say they were done back to back. A blank rep row keeps that
  exercise out of the group.
- **A deload is marked on the session**, never entered automatically: when the
  configured number of weeks has passed since the last one, the daily page says
  so and leaves the decision to the reader. A deload session's advice is half
  the last load and half its sets, named from the last session trained
  normally. The deload's rows stay in the history, marked, but the sessions
  after it compare against the normal numbers again — a planned dip is not a
  regression.
- **Effort is part of the record, not a note.** A set's effort is a row in
  `effort`, so its rank (closer to failure is a smaller number) is what the
  recommendation compares before load and reps. See
  [effort before weight](requirements.md#effort-is-compared-before-weight).
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
  of exercises, one row each, with `add exercise` growing the list and `remove`
  dropping a row; saving rewrites the preset in one transaction. A new preset
  starts with the one exercise it must have and grows from there. Finished
  sessions store the exercises they recorded, not the preset, so rewriting one
  never rewrites history.
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
scrollbar does not shift its controls sideways. Those utilities are first-party
from **Tailwind v4.3**; the build pins that release rather than whatever version
the tool happens to have cached:

```bash
$ LEPTOS_TAILWIND_VERSION=v4.3.3 cargo leptos build --release
```

cargo-leptos reads `LEPTOS_TAILWIND_VERSION`, and `test/web/drive.py` sets it, so
a build that reassembles the stylesheet gets the same release. On v4.2 the three
`scrollbar-*` classes generate nothing, and the browser falls back to its own bar.

The hydrated bundle is **inside the binary**, so installing stays one file:

```bash
$ cargo leptos build --release            # the WASM half, into target/site/pkg
$ cargo build --release --features ssr    # the binary, which embeds it
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

Screenshots and the scratch database go under `$FPR_SCRATCH_DIR`, never into the
repository.
