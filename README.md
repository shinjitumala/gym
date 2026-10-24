# gym

A local-first fitness tracker for progressive overload, body weight and food.
It records your training sets, weigh-ins and meals in plain-text [TOML][toml]
files on your machine, with no server and no account.

![Progressive overload (mixed)](i/0.jpg)
![Progressive overload (single)](i/1.jpg)
![Weight and bodyfat](i/2.jpg)

[toml]: https://toml.io

## Install

Builds and installs the `gym` binary into `$CARGO_HOME/bin` (default
`~/.cargo/bin`).

Prerequisites: `cargo` (Rust), `jq`, `rsync`, and `git` (used by `gym sync`).

```bash
$ git clone https://github.com/shinjitumala/gym.git
$ cd gym
$ ./install.sh
```

`install.sh` formats, builds with `cargo build --release --locked`, and
`rsync -ruP`s the binary into place through a temp file and an atomic rename,
so it is idempotent and safe while `gym` is running.

## Quick start

Point `gym` at a data directory and a git repository for it, in
`~/.config/gym.json`:

```json
{
  "db": "/home/me/g/gym_data/d",
  "repo": "/home/me/g/gym_data"
}
```

Both directories must already exist; see
[Configuration](doc/src/configuration.md) for the lookup order, the smallest
directory that works, and a throwaway setup.

Then log a session, a meal or a weigh-in:

```bash
$ gym new     # training sets, with the last three sessions as history
$ gym food    # meals, with the day's calories and macros
$ gym weight  # a weigh-in
```

## Commands

| Command     | What it does                                                    |
|-------------|-----------------------------------------------------------------|
| `gym new`   | A training session: place, exercises, load × reps per set.       |
| `gym food`  | Meals for the day, with running calorie and macro totals.        |
| `gym weight`| A weigh-in: kilograms, body fat, note.                           |
| `gym sync`  | Pull, commit and push the data repository.                       |
| `gym web`   | Not implemented yet: panics with `not yet implemented`.          |

Run `gym` with no command for a menu of the five. Every command is interactive
and ESC backs out of the current question.

## Links

- [Design notes](doc/src/index.md) — the mdBook in `doc/`: configuration, the
  data file schemas, each command with a worked example, and the
  [feature list with acceptance criteria](doc/src/features.md).
  Build it with `mdbook build doc`, or read it with `mdbook serve doc`.

## Deploy to Android

`deploy.sh` cross-compiles a release binary for `aarch64-linux-android` with
the Android NDK toolchain.

```bash
$ ./deploy.sh
```

## License

Proprietary. See [LICENSE](LICENSE).
