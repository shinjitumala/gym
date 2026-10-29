# How the code is written

The source follows a small set of rules. They are stated here so the next reader
does not have to infer them from a diff, and so a rule that is not written down
is not a rule.

## No comments in the code

A `///` or `//!` doc comment states an interface — what a caller gets, what an
error means — and is part of the documentation. Nothing else is a comment. A
remark about an implementation means one of two things:

- the code has two phases and wants a function name for the second, or
- a value is named so badly that a sentence is needed to explain it.

Both are fixed in the code, not noted beside it. Before, the comment names the
function that should exist:

```rust
fn step(&mut self) {
    // drop the dead bodies
    self.bodies.retain(|b| b.alive);
}
```

After, the comment is the name:

```rust
fn step(&mut self) {
    self.drop_dead();
}

fn drop_dead(&mut self) {
    self.bodies.retain(|b| b.alive);
}
```

The rules the code enforces — that a warmup is not a rung on the effort ladder,
that a set above 30 reps has no estimate, that a split offers a day's preset
instead of filling it — are product rules, and they live in
[the feature list](features.md), [the acceptance criteria](requirements.md) and
[the web UI](web.md) beside the behaviour they describe. A `//` line in the
source was a second copy of one of those, free to drift; the pages are the copy
that is read.

## One step per call

A function that does more than one thing is split until its body reads as a list
of steps, one call each, and its name says which operation it is. The import is
the clearest case: reading the archive, checking its references and writing the
rows are three operations over the same data, and the top-level function is now
those three calls:

```rust
pub async fn import(db: &Db, dir: &Path, map_path: Option<&Path>) -> Res<ImportReport> {
    let a = read_archive(dir, map_path)?;
    let mut r = ImportReport { duplicate_refs: validate(&a)?, ..Default::default() };
    let mut tx = db.pool.begin().await?;
    write_reference_tables(&mut tx, &a, &mut r).await?;
    write_sessions(&mut tx, &a, &mut r).await?;
    write_weights(&mut tx, &a, &mut r).await?;
    write_meals(&mut tx, &a, &mut r).await?;
    tx.commit().await?;
    Ok(r)
}
```

The same shape is in `src/web/data.rs`: `exercise_window` asks `deload_of`,
`exercise_index`, `finished_sessions`, `working_sessions` and `advice`, and each
of those answers one question. Reading the top-level function tells the reader
the order of the work without reading any of the queries.

A function that changes state outside its arguments returns `Result<(), _>` and
its name says what it changes: `write_meal`, `add_weight`, `activate`,
`set_span`. A function that only reads returns its value.

## One feature, one place

A module answers one question, and the types, the functions and the tests for
that question sit in it together. Pointing at the module that answers a question
should take one path:

| Question | Module |
| --- | --- |
| What should the next set be, and why? | `src/train.rs` |
| Is this trend on goal, and is a maintenance estimate supported? | `src/stats.rs` |
| How does a TOML archive become rows? | `src/import.rs` |
| What does the store offer? | `src/db.rs` |
| What does a page read? | `src/web/data.rs` |
| What does a page write? | `src/web/actions.rs` |
| Which server function does a page call? | `src/web/api.rs` |
| What does a page render? | `src/web/pages.rs` |
| How is a chart drawn? | `src/web/charts.rs` |
| What does the shell look like? | `src/web/ui.rs` |
| What crosses the wire? | `src/web/dto.rs` |
| What does a command ask? | `src/cmd/` |

The tests follow the same rule: `src/train.rs` and `src/stats.rs` carry their
rule's tests in the module, and `test/` holds the tests that need a process —
the CLI, the importer and the store.

One boundary cuts across the table, and the build makes it: `web/dto.rs` and
`web/api.rs` are declarations the browser half compiles, while `web/data.rs`,
`web/actions.rs` and `web/server.rs` are behind `#[cfg(feature = "ssr")]`. A DTO
and the server function's signature therefore live in the shared module, and the
query that fills it and the write behind it live in the server-only one. Moving
them together would put `sqlx` and the pool into the WASM bundle.

An abstraction is added at its second caller, not its first. `listed` in
`src/import.rs` exists because four report lists are formatted the same way;
`count` because nine tables are counted; `group_of` because the picker and the
chart label an exercise the same way. A helper with one caller stays inline
unless its name says what the call site could not.

## Data and systems

A struct is the layout of a value the program passes around, and the behaviour
over it is a function that takes that value. An `impl` block holding both the
container and the queries over it is an object, and it hides what a call touches.

`Db` was one: a pool with eleven query methods on it. It is now the pool and its
path, two constructors, and free functions that take it:

```rust
pub struct Db {
    pub pool: Pool<Sqlite>,
    pub path: String,
}

impl Db {
    pub async fn open(p: &str) -> Res<Self> {
        connected(existing_file(p)?, false).await
    }
}

pub async fn places(db: &Db) -> Res<Vec<Place>> { /* the query */ }
```

```rust
let places = crate::db::places(&db).await?;
```

What remains inside an `impl` is a conversion or a decision about the value
itself — `RawSet::row` copies a set into the shape a page renders,
`Change::word` names a variant, `Date` formats itself. None of them reaches for
another part of the program.

An identity is an id, never a handle: `exercise`, `session`, `food` and `effort`
are `i64` columns, and a row is looked up in the query that needs it. A closed
set of cases is an `enum` (`Change`, `Goal`, `Verdict`), never a trait with one
implementation.

## A view function is as long as its markup

`src/web/pages.rs` holds components whose bodies are one `view!` expression. The
test for a small body — no two statements separated by a blank line marking a
new phase — cannot be met by splitting one markup tree into fragments: the whole
body is one statement, and a component already answers exactly one question
("render this form"). What it means in practice:

- the calculations a page needs happen before the `view!` and are named
  (`session_counts`, `week_sets`, `prefill`, `current_food`), so the markup reads
  as values it was given;
- a piece of markup another page also draws is a component (`Section`, `Kpi`,
  `Picker`, `Outcome`), not a copy;
- a page that needs a second phase gets a helper function, not a second
  `view!`.

## Dependencies

An internal path dependency carries `version = "*"` beside its `path`. A concrete
requirement breaks a `--locked` build the moment the depended-on crate's own
version moves: every dependent still asking for the old number fails to resolve
before anything compiles, and the one that was forgotten is found in CI.

```toml
[dependencies]
fpr-core = { path = "../fpr-core", version = "*" }
```

`gym` has no internal path dependency today — `fpr-cli-util` comes from the
registry, and everything else is third-party — so there is nothing to pin. The
rule applies the first time a sibling checkout is added: write `version = "*"`,
not the number the sibling happens to carry.
