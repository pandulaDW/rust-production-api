## A newsletter API written in Rust

#### Features

- [actix-web](https://docs.rs/actix-web/4.3.1/actix_web/) is used as the web framework.

- Postgres as the db and the [sqlx](https://crates.io/crates/sqlx) for compiling and connecting and executing queries with the db.

- [tracing framework](https://docs.rs/tracing/latest/tracing/index.html) is used for logging. Logs will be formatted as JSON log lines and a middleware is used to attach a request ID for each log. [Instrument macros](https://docs.rs/tracing/latest/tracing/attr.instrument.html) are used in handlers to create trace spans.
