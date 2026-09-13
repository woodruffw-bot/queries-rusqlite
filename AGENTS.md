# Project guidance

Add dependencies only when necessary. Keep workspace and generated code free
of unsafe Rust.

Bind parameters through rusqlite. Do not interpolate values into SQL or add a
SQL parser. Follow rusqlite's connection and transaction semantics.

Test success and failure paths. Preserve 100% line, function, and region
coverage in both crates without excluding production code.

Write terse, simple technical English. Document public API behavior without
restating signatures. Keep the README to installation, a small example, and a
pointer to the API docs.

Make each commit a coherent change. Use the workflows in `.github/workflows/`
as the source of truth for checks and tool versions.
