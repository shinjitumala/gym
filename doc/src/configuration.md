# Configuration

One TOML file tells `gym` where its database lives and where `gym sync` sends a
copy. The database is one file; the tool creates it, not its directory.

## Where the file is looked up

1. `$FPR_GYM_CONFIG_PATH`, if it is set.
2. Otherwise `$HOME/.config/gym.toml`.

The variable comes first, so pointing at another file is one variable and
nothing else: no second copy in the home directory, no shell alias.

The same variable is what `gym web` reads its configuration through, so a
browser on a throwaway database needs it on the command that starts the server,
not on the one that opened the page.

`--help` and `--version` answer without the file: they describe the binary, and
a machine that has just installed it has no configuration yet.

## Fields

| Field    | Type   | Meaning                                                             |
|----------|--------|---------------------------------------------------------------------|
| `db`     | string | Path to the SQLite database **file**. Not a directory.               |
| `remote` | string | `rsync` destination for `gym sync`: a path or `host:path`.          |

`db` need not exist yet: `gym db init` creates it, and every other command
refuses to run until it does. Its directory must already exist.

## A complete example

`~/.config/gym.toml`:

```toml
db = "/home/me/g/gym_data/gym.db"
remote = "fpr0:/home/me/g/gym_data/gym.db"
```

With that file in place, `gym food` reads and writes
`/home/me/g/gym_data/gym.db`, and `gym sync` copies a snapshot of it to
`fpr0`.

A local destination works the same way, because `remote` is handed to `rsync`
verbatim:

```toml
db = "/home/me/g/gym_data/gym.db"
remote = "/mnt/backup/gym.db"
```

## A throwaway configuration

Useful for trying a command without touching real data:

```bash
$ mkdir -p /tmp/gym-try/backup
$ printf 'db = "/tmp/gym-try/gym.db"\nremote = "/tmp/gym-try/backup/gym.db"\n' \
    > /tmp/gym-try/gym.toml
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/gym.toml gym db init
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/gym.toml gym weight
```

`gym db init` writes the schema, the seeded effort presets and the four
settings, so the second command has something to write into. To fill a
throwaway database with the real log instead, see
[the schema page](sqlite-schema.md#migrating-the-toml-archive).

## Errors

Configuration is validated before any command runs, and a failure ends the
process with exit status 1.

```text
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/missing.toml gym food
Failed to open file '/tmp/gym-try/missing.toml' because 'No such file or directory (os error 2)'.
Aborting.
```

```text
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/gym.toml gym food   # db points at a directory
'db' is '/tmp/gym-try', a directory, but it must be the path to the database file.
Aborting.
```

```text
$ FPR_GYM_CONFIG_PATH=/tmp/gym-try/gym.toml gym food   # the database file is absent
No database at '/tmp/gym-try/gym.db'. Create one with 'gym db init', or import the TOML archive with 'gym db import <dir>'.
Aborting.
```

| Symptom | Cause |
|---------|-------|
| `Failed to get variable '$HOME'.` | Neither the variable nor `$HOME` is set, so there is no path to try. |
| `Failed to open file '<path>' ...` | The file named by `FPR_GYM_CONFIG_PATH`, or by `$HOME/.config/gym.toml`, does not exist or cannot be read. |
| `Failed to parse '<path>' ...` | The file is not TOML, or a field is missing or not a string. |
| `'db' is '<path>', a directory, ...` | `db` is a directory; the database is a file. |
| `'db' is '<path>', whose directory does not exist.` | The file's parent is missing. |
| `'remote' is empty; ...` | `remote` is blank, so `gym sync` would have nowhere to write. |
| `No database at '<path>'. ...` | `gym db init` has not been run. |
