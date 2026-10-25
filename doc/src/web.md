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

## Six pages, two navigation rows

The bar is two rows because the six pages are a grid: **food** or **gym** on the
first row, **daily**, **edit** or **analysis** on the second. On a phone the
rows scroll sideways instead of wrapping, so the bar keeps one height.

| Domain | Daily | Edit | Analysis |
|--------|-------|------|----------|
| Food | Record meals and weigh-ins; today's totals; the trend. | Meals, weigh-ins, foods, presets. | Intake split by food, the trend, the rankings. |
| Gym | Record a session; switch phase; the deload reminder. | Sets, exercises, places, muscle groups. | One exercise's strength trend; sets per muscle group. |

`/status` shows the database file, the remote, and the row counts.

A page with more than one subject — meals and weight, sets and exercises — puts
them behind a **segmented switch** rather than a route, so the six routes stay
six and a phone never gets a third navigation bar.

Each page is one query against the server, so it shows what the database held
when it was asked. A form that saves bumps a counter and the page re-runs its
query, so the row you just wrote is there.

## Recording

- **A meal is one screen, three rows.** Fill as many rows as the meal has
  parts; a row left on `—` is not recorded, so the common case is still one
  click. The date defaults to today and **now** fills the time from the
  server's clock.
- **A preset is one button.** A preset is a named set of foods with amounts;
  tapping it records every item at once, at the current date and time. They are
  defined on `edit → presets`, and the daily tab lists them by last use.
- **A weigh-in is separate from a meal.** Body fat left blank is stored as
  `NULL`, which is not the same as a measured `0`.
- **A session is four exercise rows.** A row left empty is not recorded.
  Picking an exercise asks for the last three sessions of *that exercise at
  that place* and shows them under the form, so the numbers to beat are one
  glance away instead of a wall of history.
- **The phase switch is on the gym daily tab.** Starting a phase ends the old
  one from that date; past sessions keep the phase they were trained in.

## Editing

Every editing row opens in place: **edit** folds out the fields, **save** writes
them, **delete** removes the row. Deleting a set closes the gap in its session's
order. A food or exercise still referenced by a row cannot be silently deleted:
the refusal says how many rows use it.

- **Merging an exercise** moves every set and list item to the survivor, unions
  the muscle groups, and deletes the other. There is no undo, so the button
  says so.
- **A deload is a session's flag**, not a set's: the toggle sits on the session
  row, because a deload is a week's plan.

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

The hydrated bundle is **inside the binary**, so installing stays one file:

```bash
$ cargo leptos build --release            # the WASM half, into target/site/pkg
$ cargo build --release --features ssr    # the binary, which embeds it
```

`./install.sh` runs both in that order. The second pass matters: `cargo leptos`
starts the binary's build before the site is complete, so that pass can embed
nothing, and recompiling the binary is what puts the assets in.

A build without the bundle — a plain `cargo build` — is still a working CLI;
`gym web` then refuses to start and says to rebuild, instead of serving an empty
page.

## Why the forms read the DOM

A field's value is read from the element when the form is submitted, not from a
signal mirrored on every keystroke. The element is the source of truth: a mirror
can lag (autofill, IME composition, a test harness setting the value), and a
mount that re-renders a reactive `value` can overwrite what was typed. The
fields are therefore uncontrolled — an initial `value` and nothing reactive —
and the submit reads them.

The server functions take a JSON body (`#[server(input = Json)]`). The default
codec is URL-encoded, which cannot carry a list of rows, and a form here
regularly sends one — the meal rows, a session's sets, a set of muscle groups.

## Caching

Every response carries `Cache-Control: no-cache`. The bundle's URLs are stable
across builds, so a browser that kept a script and paired it with a newly built
WASM would hydrate two different DOM shapes and panic inside `tachys`.
`no-cache` still allows a 304; it only forces the check, so half of a previous
build cannot survive an install.
