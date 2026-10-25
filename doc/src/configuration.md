# Configuration

> **What this page describes.** The config of the running build. In
[the overhaul](overhaul.md) `db` becomes the path to one SQLite file — not a
directory — and `repo` becomes `remote`, one rsync destination: a local path or
`host:path`.

One JSON file tells `gym` where the data directory and its git repository
live. Both directories must already exist: the tool creates files inside them,
never the directories themselves.

## Where the file is looked up

1. `$HOME/.config/gym.json`, if it exists.
2. Otherwise `$CONFIG_PATH`.

The first rule wins unconditionally. Once `~/.config/gym.json` exists,
`CONFIG_PATH` is ignored — which is what makes a throwaway configuration
possible: point `HOME` at an empty directory as well, so the lookup falls
through to `CONFIG_PATH`.

## Fields

| Field  | Type   | Meaning                                              |
|--------|--------|------------------------------------------------------|
| `db`   | string | Directory holding the TOML data. Must exist.         |
| `repo` | string | Git repository `gym sync` pulls, commits and pushes. Must exist. |

Both are absolute paths in practice; they are used as given.

## A complete example

`~/.config/gym.json`:

```json
{
  "db": "/home/me/g/gym_data/d",
  "repo": "/home/me/g/gym_data"
}
```

With that file in place, `gym food` reads and writes under
`/home/me/g/gym_data/d`, and `gym sync` runs git in `/home/me/g/gym_data`.

## A throwaway configuration

Useful for trying a command without touching real data:

```bash
$ mkdir -p /tmp/gym-try/db/{set,session,weight,meal}
$ printf '[e]\n' > /tmp/gym-try/db/exercise.toml
$ printf '[e]\n' > /tmp/gym-try/db/muscle_group.toml
$ printf '[e]\n' > /tmp/gym-try/db/place.toml
$ printf '[e]\n' > /tmp/gym-try/db/food.toml
$ mkdir -p /tmp/gym-try/home /tmp/gym-try/repo
$ printf '{"db":"/tmp/gym-try/db","repo":"/tmp/gym-try/repo"}' > /tmp/gym-try/gym.json
$ HOME=/tmp/gym-try/home CONFIG_PATH=/tmp/gym-try/gym.json gym weight
```

The empty `[e]` tables are not decoration: a missing or empty `*.toml` file is
read as "no database" and the command then aborts when it tries to save. See
[the data files](data-files.md#the-smallest-directory-gym-accepts) for why.

## Errors

Configuration is validated before any command runs, and a failure ends the
process with exit status 1. The messages are:

```text
$ CONFIG_PATH=/tmp/gym-try/missing.json gym food
Failed to read file '/tmp/gym-try/missing.json' because 'No such file or directory (os error 2)'
Aborting.
```

```text
$ CONFIG_PATH=/tmp/gym-try/gym.json gym food   # db points at a missing directory
Database path does not exist '/tmp/gym-try/db'
Aborting.
```

| Symptom | Cause |
|---------|-------|
| `'CONFIG_PATH' not set.` | Neither `~/.config/gym.json` nor `CONFIG_PATH` exists. |
| `Failed to read file '<path>' ...` | The resolved file is unreadable. |
| `Failed to parse rules from '<path>' ...` | The file is not valid JSON, or a field is missing or not a string. |
| `Database path does not exist '<path>'` | `db` is not a directory. |
| `Data repository path does not exist '<path>'` | `repo` is not a directory. |
