# Working on queries-rusqlite

Keep the crate small. Add dependencies only when the implementation requires
them. Keep `unsafe_code = "forbid"` and `missing_docs = "deny"` in the workspace
Cargo.toml, inherited by both crates. Do not duplicate them as crate attributes.
Document public APIs in terse, simple technical English.

Keep both crates under `crates/`. The root Cargo.toml defines the workspace;
run workspace checks from the repository root.

Bind SQL parameters through rusqlite. Do not interpolate values into SQL or add
a SQL parser. Follow rusqlite's connection and transaction semantics.

Test successful queries and failures, including row counts, conversion errors,
and transaction behavior. Keep full line, function, and region coverage for both
crates. Do not exclude production code to meet the coverage requirement.

Make each commit a coherent change with a short, descriptive message.

Write plain English. Explain behavior and limitations directly. Avoid marketing
language, exaggerated claims, and comments that restate the code.

Keep README.md focused on installation and using the crate. Put implementation
policy, contributor instructions, and CI commands here.

## Dependencies and safety

The runtime uses rusqlite and the companion macro crate. The macro crate uses
`proc-macro2`, `quote`, and `syn`; there are no test dependencies. Keep rusqlite's
default features disabled and link system SQLite unless a change requires
otherwise.

The unsafe-code prohibition applies to workspace source and generated code.
Rusqlite and its SQLite bindings use unsafe code for FFI.

## Checks

Before committing, run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
cargo test --workspace --doc --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
```

CI tests Rust 1.95.0 and stable. Coverage uses Rust 1.95.0 and cargo-llvm-cov:

```sh
rustup toolchain install 1.95.0 --profile minimal --component llvm-tools-preview
cargo +1.95.0 install cargo-llvm-cov --locked --version 0.9.1
cargo +1.95.0 llvm-cov --workspace --all-targets --locked \
  --ignore-filename-regex '(/tests/|/src/tests\.rs$)' \
  --fail-under-lines 100 --fail-under-functions 100 --fail-under-regions 100
```

Exclude only test files from coverage. Test documentation examples separately;
coverage does not establish correctness.

CI runs zizmor 1.30.1 in pedantic mode and fails on findings. With zizmor
installed, run:

```sh
zizmor --persona=pedantic .
```

Set `GH_TOKEN` to include online audits, as CI does.
