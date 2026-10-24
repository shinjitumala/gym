# The data files

The `db` directory is the database. `gym` reads the files it needs on each
command and appends rows to them; it never creates a directory and never
deletes a row.

## Layout

```text
<db>/
  exercise.toml          every exercise ever logged
  muscle_group.toml      muscle groups, referenced by exercises
  place.toml             gyms, referenced by sessions
  food.toml              foods, referenced by meals
  set/YYYY-MM-DD.toml    one file per UTC day
  session/YYYY-MM.toml   one file per UTC month
  weight/YYYY-MM.toml    one file per UTC month
  meal/YYYY-MM-DD.toml   one file per UTC day
```

The four `*.toml` index files and the four sub-directories must all exist
before the first command runs. A missing sub-directory fails immediately:

```text
$ HOME=/tmp/gym-try/home CONFIG_PATH=/tmp/gym-try/gym.json gym food
Failed to run:
Failed to read '/tmp/gym-try/db/set' because 'No such file or directory (os error 2)'
Aborting.
```

## Row format

Every file holds one TOML table named `e`, keyed by an integer id, one row per
table:

```toml
[e.0]
name = "bench press"
desc = ""
muscle_groups = []

[e.1]
name = "lat pull down"
desc = "wide grip"
muscle_groups = []
```

Ids are plain integers with no other meaning. In the index files they are
`0, 1, 2, …` in insertion order. In the sharded directories a new id is one
more than the largest id found across all files of that directory, so ids stay
unique as files are added, and the first sharded row is `[e.1]`.

## Schemas

`exercise.toml`

| Field           | Type              | Meaning                          |
|-----------------|-------------------|----------------------------------|
| `name`          | string            | Shown in the exercise prompt.    |
| `desc`          | string            | Free text.                       |
| `muscle_groups` | array of tables   | `group` (id) and `amount`.       |

`muscle_group.toml`, `place.toml`: `name` and `desc`.

`food.toml`

```toml
[e.0]
name = "boiled egg"
calories = 80.0
protein = 6.63
fat = 5.51
carbohydrate = 0.16
desc = ""
```

The four numbers describe one unit of the food. A meal stores how many units
were eaten, so totals are `amount × calories` and so on. Protein, fat and
carbohydrate may be left unknown when a food is registered; they are then
`0.0`.

`set/`, one training set:

```toml
[e.9649]
date = "2026-05-13T21:33:00.506835755+00:00"
session = 367
exercise = 7
load = 20.0
rep = 12.0
desc = ""
```

`session/`, one training session; `sets` lists the ids of the sets that belong
to it, and `place` is a `place.toml` id:

```toml
[e.361]
date = "2026-05-04T06:14:45+09:00"
place = 5
sets = [9550, 9551, 9552]
desc = ""
```

`weight/`, one weigh-in:

```toml
[e.302]
date = "2026-04-01T22:18:45+00:00"
kg = 69.6
bodyfat = 13.7
desc = ""
```

`meal/`, one meal; `food` is a `food.toml` id and `amount` multiplies its
numbers:

```toml
[e.1]
date = "2026-10-24T23:53:47+00:00"
food = 0
amount = 2.0
desc = "breakfast"
```

Every `date` is RFC3339. Sessions and weigh-ins keep the offset the date was
entered with; sets and meals are written in UTC, because they are stamped with
the clock rather than asked for.

## Which file a row lands in

Shard names come from the row's timestamp converted to UTC:

- `set/` and `meal/`: the UTC date. A meal eaten at 08:53 in `+09:00` is
  `2026-10-24T23:53:47+00:00` and lands in `meal/2026-10-24.toml`.
- `session/` and `weight/`: the UTC year and month.

The consequence worth remembering: a query for "today" uses the local day
(midnight to midnight, as `gym food` does), while the rows for it may be split
across two files.

## Hand editing

The files are meant to be readable and editable, with two rules:

- **Append rows; do not renumber them.** A session's `sets` list and a meal's
  `food` reference are ids, and index-file ids are the entry count, so
  deleting a row can make the next row reuse its id and silently rebind a
  reference.
- **Comments survive only in files the tool does not touch.** A command
  rewrites a shard file wholesale when it appends to it, so comments and
  formatting in that file are lost; rows in files it never writes are left
  alone.

### The smallest directory `gym` accepts

The four index files must exist *and* parse. Each must contain at least the
empty table `e`:

```text
[e]
```

An empty (0-byte) file or a missing one is read as "no database", and the
first save then fails, because the tool no longer knows the path it was meant
to write:

```text
$ HOME=/tmp/gym-try/home CONFIG_PATH=/tmp/gym-try/gym.json gym food
Failed to run:
Failed to write to '' because 'No such file or directory (os error 2)'.
Aborting.
```

See [Configuration](configuration.md#a-throwaway-configuration) for a script
that builds such a directory.
