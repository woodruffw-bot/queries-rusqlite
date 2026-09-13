# Working on queries-rusqlite

Keep the crate small. Add dependencies only when the implementation requires
them. Use safe Rust, and preserve `forbid(unsafe_code)` in both crates.

Bind SQL parameters through rusqlite. Do not interpolate values into SQL or add
a SQL parser. Follow rusqlite's connection and transaction semantics.

Test successful queries and failures, including row counts, conversion errors,
and transaction behavior. Keep full line, function, and region coverage for both
crates. Do not exclude production code to meet the coverage requirement.

Before committing, run the checks described in README.md. Make each commit a
coherent change with a short, descriptive message.

Write plain English. Explain behavior and limitations directly. Avoid marketing
language, exaggerated claims, and comments that restate the code.
