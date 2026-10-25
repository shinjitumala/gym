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

## The six tabs

| Route | Tab | What it is for |
|-------|-----|----------------|
| `/food` | Food · daily | Record meals and weigh-ins; today's totals; the weight trend against the phase goal. |
| `/food/edit` | Food · intervention | Every meal, weigh-in and food from the last 120 days, to edit or delete. |
| `/food/analysis` | Food · analysis | Intake by day split by food, the weight trend, and the foods ranked by frequency and by calories. |
| `/gym` | Gym · daily | The last eight sessions with their sets, a deload reminder, and the PRs each session set. |
| `/gym/edit` | Gym · intervention | Exercises with their muscle groups and video links, places, muscle groups, and recent sessions. |
| `/gym/analysis` | Gym · analysis | Best estimated 1RM per exercise per session, grouped by body part; sets per muscle group per week; ungrouped exercises. |

`/status` shows the database file, the remote, and the row counts.

Each tab is one query against the server, so a page shows what the database held
when it was asked. A form that saves bumps a counter and the page re-runs its
query, so the row you just wrote is there.

## What the numbers mean

The statistics are the part with rules, and they are on
[the overhaul](overhaul.md#weight-and-intake-statistics) page. In short:

- A slope needs enough weigh-ins in the window, and the page says how many are
  missing rather than drawing a line from two mornings.
- A goal is met only when the trend's whole uncertainty band is inside the goal
  range; a band that straddles the boundary says "too noisy to say".
- Maintenance is withheld until enough days were actually logged, because an
  unlogged dinner biases it downwards.
- Days, never foods, are flagged when intake and weight disagree.

## Building it

The hydrated bundle is **inside the binary**, so installing stays one file:

```bash
$ cargo leptos build --release   # the WASM half, into target/site/pkg
$ cargo build --release --features ssr   # the binary, which embeds it
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

## Caching

Every response carries `Cache-Control: no-cache`. The bundle's URLs are stable
across builds, so a browser that kept a script and paired it with a newly built
WASM would hydrate two different DOM shapes and panic inside `tachys`.
`no-cache` still allows a 304; it only forces the check, so half of a previous
build cannot survive an install.
