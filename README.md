# sqlite3builder

[![Build Status](https://travis-ci.org/perdumonocle/sqlite3builder.svg)](https://travis-ci.org/perdumonocle/sqlite3builder)
[![Latest Version](https://img.shields.io/crates/v/sqlite3builder.svg)](https://crates.io/crates/sqlite3builder)
[![Docs](https://docs.rs/sqlite3builder/badge.svg)](https://docs.rs/sqlite3builder)

Simple SQL code generator.

## Usage

To use `sqlite3builder`, first add this to your `Cargo.toml`:

```toml
[dependencies]
sqlite3builder = "0.2"
```

Next, add this to your crate:

```rust
extern crate sql_builder;

use sql_builder::SqlBuilder;
```

Example:

```rust
let sql = SqlBuilder::select_from("company")
    .field("id")
    .field("name")
    .and_where("salary > 25000")
    .sql()?;

assert_eq!("SELECT id, name FROM company WHERE salary > 25000;", &sql);
```

## SQL support

### Statements

- SELECT
- INSERT
- UPDATE
- DELETE

### Operations

- join
- distinct
- group by
- order by
- where
- limit, offset
- subquery

### Functions

- escape
- query

## License

This project is licensed under the [MIT license](LICENSE).
