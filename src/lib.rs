//! Simple SQL code generator. May be used with pooled Sqlite3 connection.
//!
//! ## Usage
//!
//! To use `sqlite3builder`, first add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! sqlite3builder = "0.3"
//! ```
//!
//! Next, add this to your crate:
//!
//! ```
//! extern crate sqlite3builder;
//!
//! use sqlite3builder::Sqlite3Builder;
//! ```
//!
//! # Example:
//!
//! ```
//! extern crate sqlite3builder;
//!
//! # use std::error::Error;
//! use sqlite3builder::Sqlite3Builder;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! let sql = Sqlite3Builder::select_from("company")
//!     .field("id")
//!     .field("name")
//!     .and_where("salary > 25000")
//!     .sql()?;
//!
//! assert_eq!("SELECT id, name FROM company WHERE salary > 25000;", &sql);
//! # Ok(())
//! # }
//! ```
//!
//! ## SQL support
//!
//! ### Statements
//!
//! - SELECT
//! - INSERT
//! - UPDATE
//! - DELETE
//!
//! ### Operations
//!
//! - join
//! - distinct
//! - group by
//! - order by
//! - where
//! - limit, offset
//! - subquery
//! - get all results
//! - get first row
//! - get first value, first integer value, first string value
//!
//! ### Functions
//!
//! - escape
//! - query
//!
//! ## License
//!
//! This project is licensed under the [MIT license](LICENSE).

#[macro_use]
extern crate log;
extern crate sql_builder;

use serde_json::value::Value as JValue;
use sql_builder::{esc as SqlBuilderEsc, quote as SqlBuilderQuote, SqlBuilder};
use sqlite3::Cursor;
use sqlite3::Value as SValue;
use std::error::Error;

/// Pooled Sqlite3 connection
type ConnPooled = r2d2::PooledConnection<r2d2_sqlite3::SqliteConnectionManager>;

/// Main Sqlite3 builder
pub struct Sqlite3Builder {
    builder: SqlBuilder,
}

impl Sqlite3Builder {
    /// Create SELECT query.
    /// You may specify comma separted list of tables.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("price > 100")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');", &sql);
    /// // add                               ^^^^^
    /// // here                              table
    /// # Ok(())
    /// # }
    /// ```
    pub fn select_from<S: ToString>(table: S) -> Self {
        Self {
            builder: SqlBuilder::select_from(table),
        }
    }

    /// Create SELECT query without a table.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_values(&["10", &quote("100")])
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT 10, '100';", &sql);
    /// // add             ^^^^^^^^^
    /// // here             values
    /// # Ok(())
    /// # }
    /// ```
    pub fn select_values<S: ToString>(values: &[S]) -> Self {
        Self {
            builder: SqlBuilder::select_values(values),
        }
    }

    /// Create INSERT query.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .values(&[quote("In Search of Lost Time"), 150.to_string()])
    ///     .values(&["'Don Quixote', 200"])
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);", &sql);
    /// // add                  ^^^^^
    /// // here                 table
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert_into<S: ToString>(table: S) -> Self {
        Self {
            builder: SqlBuilder::insert_into(table),
        }
    }

    /// Create UPDATE query.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::update_table("books")
    ///     .set("price", "price + 10")
    ///     .sql()?;
    ///
    /// assert_eq!("UPDATE books SET price = price + 10;", &sql);
    /// // add             ^^^^^
    /// // here            table
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_table<S: ToString>(table: S) -> Self {
        Self {
            builder: SqlBuilder::update_table(table),
        }
    }

    /// Create DELETE query.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::delete_from("books")
    ///     .and_where("price > 100")
    ///     .sql()?;
    ///
    /// assert_eq!("DELETE FROM books WHERE price > 100;", &sql);
    /// // add                  ^^^^^
    /// // here                 table
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_from<S: ToString>(table: S) -> Self {
        Self {
            builder: SqlBuilder::delete_from(table),
        }
    }

    /// Use NATURAL JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL JOIN orders;", &sql);
    /// // add here                                ^^^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn natural(&mut self) -> &mut Self {
        self.builder.natural();
        self
    }

    /// Use LEFT JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .left()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL LEFT JOIN orders;", &sql);
    /// // add here                                        ^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn left(&mut self) -> &mut Self {
        self.builder.left();
        self
    }

    /// Use LEFT OUTER JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .left_outer()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL LEFT OUTER JOIN orders;", &sql);
    /// // add here                                        ^^^^^^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn left_outer(&mut self) -> &mut Self {
        self.builder.left_outer();
        self
    }

    /// Use RIGHT JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .right()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL RIGHT JOIN orders;", &sql);
    /// // add here                                        ^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn right(&mut self) -> &mut Self {
        self.builder.right();
        self
    }

    /// Use INNER JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .inner()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL INNER JOIN orders;", &sql);
    /// // add here                                        ^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn inner(&mut self) -> &mut Self {
        self.builder.inner();
        self
    }

    /// Use CROSS JOIN
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("total")
    ///     .natural()
    ///     .cross()
    ///     .join("orders")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, total FROM books NATURAL CROSS JOIN orders;", &sql);
    /// // add here                                        ^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn cross(&mut self) -> &mut Self {
        self.builder.cross();
        self
    }

    /// Join with table.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books AS b")
    ///     .field("b.title")
    ///     .field("s.total")
    ///     .left()
    ///     .join("shops AS s")
    ///     .on("b.id = s.book")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT b.title, s.total FROM books AS b LEFT JOIN shops AS s ON b.id = s.book;", &sql);
    /// // add                                                        ^^^^^^^^^^
    /// // here                                                         table
    /// # Ok(())
    /// # }
    /// ```
    pub fn join<S: ToString>(&mut self, table: S) -> &mut Self {
        self.builder.join(table);
        self
    }

    /// Join constraint to the last JOIN part.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books AS b")
    ///     .field("b.title")
    ///     .field("s.total")
    ///     .join("shops AS s")
    ///     .on("b.id = s.book")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT b.title, s.total FROM books AS b JOIN shops AS s ON b.id = s.book;", &sql);
    /// // add                                                                 ^^^^^^^^^^^^^
    /// // here                                                                 constraint
    /// # Ok(())
    /// # }
    /// ```
    pub fn on<S: ToString>(&mut self, constraint: S) -> &mut Self {
        self.builder.on(constraint);
        self
    }

    /// Set DISTINCT for fields.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .distinct()
    ///     .field("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT DISTINCT price FROM books;", &sql);
    /// // add here        ^^^^^^^^
    /// # Ok(())
    /// # }
    /// ```
    pub fn distinct(&mut self) -> &mut Self {
        self.builder.distinct();
        self
    }

    /// Add fields.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .fields(&["title", "price"])
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books;", &sql);
    /// // add             ^^^^^^^^^^^^
    /// // here               fields
    /// # Ok(())
    /// # }
    /// ```
    pub fn fields<S: ToString>(&mut self, fields: &[S]) -> &mut Self {
        self.builder.fields(fields);
        self
    }

    /// Replace fields.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    /// # #[derive(Default)]
    /// # struct ReqData { filter: Option<String>, price_min: Option<u64>, price_max: Option<u64>,
    /// # limit: Option<usize>, offset: Option<usize> }
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// # let req_data = ReqData::default();
    /// // Prepare query for total count
    ///
    /// let mut db = Sqlite3Builder::select_from("books");
    ///
    /// db.field("COUNT(id)");
    ///
    /// if let Some(filter) = &req_data.filter {
    ///   db.and_where_like_any("LOWER(title)", filter.to_lowercase());
    /// }
    ///
    /// if let Some(price_min) = &req_data.price_min {
    ///   db.and_where_ge("price", price_min);
    /// }
    ///
    /// if let Some(price_max) = &req_data.price_max {
    ///   db.and_where_le("price", price_max);
    /// }
    ///
    /// let sql_count = db.sql()?;
    /// println!("Database query: total_count: {}", &sql_count);
    ///
    /// // Prepare query for results
    ///
    /// db.set_fields(&["id", "title", "price"]);
    ///
    /// if let (Some(limit), Some(offset)) = (req_data.limit, req_data.offset) {
    ///   db.limit(limit).offset(offset);
    /// }
    ///
    /// let sql_results = db.sql()?;
    /// println!("Database query: results: {}", &sql_results);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_fields<S: ToString>(&mut self, fields: &[S]) -> &mut Self {
        self.builder.set_fields(fields);
        self
    }

    /// Add field.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books;", &sql);
    /// // add             ^^^^^  ^^^^^
    /// // here            field  field
    /// # Ok(())
    /// # }
    /// ```
    pub fn field<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.field(field);
        self
    }

    /// Replace fields with choosed one.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    /// # #[derive(Default)]
    /// # struct ReqData { filter: Option<String>, price_min: Option<u64>, price_max: Option<u64>,
    /// # limit: Option<usize>, offset: Option<usize> }
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// # let req_data = ReqData::default();
    /// // Prepare query for total count
    ///
    /// let mut db = Sqlite3Builder::select_from("books");
    ///
    /// db.field("COUNT(id)");
    ///
    /// if let Some(filter) = &req_data.filter {
    ///   db.and_where_like_any("LOWER(title)", filter.to_lowercase());
    /// }
    ///
    /// if let Some(price_min) = &req_data.price_min {
    ///   db.and_where_ge("price", price_min);
    /// }
    ///
    /// if let Some(price_max) = &req_data.price_max {
    ///   db.and_where_le("price", price_max);
    /// }
    ///
    /// let sql_count = db.sql()?;
    /// println!("Database query: total_count: {}", &sql_count);
    ///
    /// // Prepare query for results
    ///
    /// db.set_field("id");
    /// db.field("title");
    /// db.field("price");
    ///
    /// if let (Some(limit), Some(offset)) = (req_data.limit, req_data.offset) {
    ///   db.limit(limit).offset(offset);
    /// }
    ///
    /// let sql_results = db.sql()?;
    /// println!("Database query: results: {}", &sql_results);
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_field<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.set_field(field);
        self
    }

    /// Add SET part (for UPDATE).
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::update_table("books")
    ///     .set("price", "price + 10")
    ///     .sql()?;
    ///
    /// assert_eq!("UPDATE books SET price = price + 10;", &sql);
    /// // add                       ^^^^^   ^^^^^^^^^^
    /// // here                      field     value
    /// # Ok(())
    /// # }
    /// ```
    pub fn set<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.set(field, value);
        self
    }

    /// Add SET part with escaped string value (for UPDATE).
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::update_table("books")
    ///     .set_str("comment", "Don't distribute!")
    ///     .and_where_le("price", "100")
    ///     .sql()?;
    ///
    /// assert_eq!("UPDATE books SET comment = 'Don''t distribute!' WHERE price <= 100;", &sql);
    /// // add                       ^^^^^^^    ^^^^^^^^^^^^^^^^^^
    /// // here                       field           value
    /// # Ok(())
    /// # }
    /// ```
    pub fn set_str<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.set_str(field, value);
        self
    }

    /// Add VALUES part (for INSERT).
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .values(&[quote("In Search of Lost Time"), 150.to_string()])
    ///     .values(&["'Don Quixote', 200"])
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);", &sql);
    /// // add                                               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^    ^^^^^^^^^^^^^^^^^^
    /// // here                                                         values                      values
    /// # Ok(())
    /// # }
    /// ```
    pub fn values<S: ToString>(&mut self, values: &[S]) -> &mut Self {
        self.builder.values(values);
        self
    }

    /// Add SELECT part (for INSERT).
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let query = Sqlite3Builder::select_from("warehouse")
    ///     .field("title")
    ///     .field("preliminary_price * 2")
    ///     .query()?;
    ///
    /// assert_eq!("SELECT title, preliminary_price * 2 FROM warehouse", &query);
    ///
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .select(&query)
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) SELECT title, preliminary_price * 2 FROM warehouse;", &sql);
    /// // add                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                                            query
    /// # Ok(())
    /// # }
    /// ```
    pub fn select<S: ToString>(&mut self, query: S) -> &mut Self {
        self.builder.select(query);
        self
    }

    /// Add GROUP BY part.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .field("COUNT(price) AS cnt")
    ///     .group_by("price")
    ///     .order_desc("cnt")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price, COUNT(price) AS cnt FROM books GROUP BY price ORDER BY cnt DESC;", &sql);
    /// // add                                                            ^^^^^
    /// // here                                                           field
    /// # Ok(())
    /// # }
    /// ```
    pub fn group_by<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.group_by(field);
        self
    }

    /// Add HAVING condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .field("COUNT(price) AS cnt")
    ///     .group_by("price")
    ///     .having("price > 100")
    ///     .order_desc("cnt")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price, COUNT(price) AS cnt FROM books GROUP BY price HAVING price > 100 ORDER BY cnt DESC;", &sql);
    /// // add                                                                         ^^^^^^^^^^^
    /// // here                                                                           cond
    /// # Ok(())
    /// # }
    /// ```
    pub fn having<S: ToString>(&mut self, cond: S) -> &mut Self {
        self.builder.having(cond);
        self
    }

    /// Add WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("price > 100")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');", &sql);
    /// // add                                            ^^^^^^^^^^^       ^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                              cond                      cond
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where<S: ToString>(&mut self, cond: S) -> &mut Self {
        self.builder.and_where(cond);
        self
    }

    /// Add WHERE condition for equal parts.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_eq("title", &quote("Harry Potter and the Philosopher's Stone"))
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title = 'Harry Potter and the Philosopher''s Stone';", &sql);
    /// // add                                    ^^^^^   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                   field                      value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_eq<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_eq(field, value);
        self
    }

    /// Add WHERE condition for non-equal parts.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_ne("title", &quote("Harry Potter and the Philosopher's Stone"))
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title <> 'Harry Potter and the Philosopher''s Stone';", &sql);
    /// // add                                    ^^^^^    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                   field                       value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_ne<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_ne(field, value);
        self
    }

    /// Add WHERE condition for field greater than value.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_gt("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price > 300;", &sql);
    /// // add                                           ^^^^^   ^^^
    /// // here                                          field  value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_gt<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_gt(field, value);
        self
    }

    /// Add WHERE condition for field not less than value.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_ge("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price >= 300;", &sql);
    /// // add                                           ^^^^^    ^^^
    /// // here                                          field   value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_ge<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_ge(field, value);
        self
    }

    /// Add WHERE condition for field less than value.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_lt("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price < 300;", &sql);
    /// // add                                           ^^^^^   ^^^
    /// // here                                          field  value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_lt<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_lt(field, value);
        self
    }

    /// Add WHERE condition for field not greater than value.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_le("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price <= 300;", &sql);
    /// // add                                           ^^^^^    ^^^
    /// // here                                          field   value
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_le<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_le(field, value);
        self
    }

    /// Add WHERE LIKE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_like("title", "%Philosopher's%")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '%Philosopher''s%';", &sql);
    /// // add                                    ^^^^^       ^^^^^^^^^^^^^^^^
    /// // here                                   field             mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_like<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_like(field, mask);
        self
    }

    /// Add WHERE LIKE %condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_like_right("title", "Stone")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '%Stone';", &sql);
    /// // add                                    ^^^^^        ^^^^^
    /// // here                                   field        mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_like_right<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_like_right(field, mask);
        self
    }

    /// Add WHERE LIKE condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE 'Harry%';", &sql);
    /// // add                                    ^^^^^       ^^^^^
    /// // here                                   field       mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_like_left<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_like_left(field, mask);
        self
    }

    /// Add WHERE LIKE %condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_like_any("title", " and ")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '% and %';", &sql);
    /// // add                                    ^^^^^        ^^^^^
    /// // here                                   field        mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_like_any<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_like_any(field, mask);
        self
    }

    /// Add WHERE NOT LIKE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .and_where_not_like("title", "%Alice's%")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE title NOT LIKE '%Alice''s%';", &sql);
    /// // add                                    ^^^^^           ^^^^^^^^^^
    /// // here                                   field              mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_not_like<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_not_like(field, mask);
        self
    }

    /// Add WHERE NOT LIKE %condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_not_like_right("title", "Stone")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE '%Stone';", &sql);
    /// // add                                    ^^^^^            ^^^^^
    /// // here                                   field            mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_not_like_right<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_not_like_right(field, mask);
        self
    }

    /// Add WHERE NOT LIKE condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_not_like_left("title", "Harry")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE 'Harry%';", &sql);
    /// // add                                    ^^^^^           ^^^^^
    /// // here                                   field           mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_not_like_left<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_not_like_left(field, mask);
        self
    }

    /// Add WHERE NOT LIKE %condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_not_like_any("title", " and ")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE '% and %';", &sql);
    /// // add                                    ^^^^^            ^^^^^
    /// // here                                   field            mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_not_like_any<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.and_where_not_like_any(field, mask);
        self
    }

    /// Add WHERE IS NULL condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .and_where_is_null("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE price IS NULL;", &sql);
    /// // add                                    ^^^^^
    /// // here                                   field
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_is_null<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.and_where_is_null(field);
        self
    }

    /// Add WHERE IS NOT NULL condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .and_where_is_not_null("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE price IS NOT NULL;", &sql);
    /// // add                                    ^^^^^
    /// // here                                   field
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_is_not_null<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.and_where_is_not_null(field);
        self
    }

    /// Add OR condition to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("price < 10")
    ///     .or_where("price > 1000")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price < 10 OR price > 1000;", &sql);
    /// // add                                                         ^^^^^^^^^^^^
    /// // here                                                            cond
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where<S: ToString>(&mut self, cond: S) -> &mut Self {
        self.builder.or_where(cond);
        self
    }

    /// Add OR condition of equal parts to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .and_where_eq("title", &quote("Harry Potter and the Philosopher's Stone"))
    ///     .or_where_eq("title", &quote("Harry Potter and the Chamber of Secrets"))
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title = 'Harry Potter and the Philosopher''s Stone' OR title = 'Harry Potter and the Chamber of Secrets';", &sql);
    /// // add                                                                                           ^^^^^   ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                                                                          field                     value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_eq<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_eq(field, value);
        self
    }

    /// Add OR condition of non-equal parts to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_ne("title", &quote("Harry Potter and the Philosopher's Stone"))
    ///     .or_where_ne("title", &quote("Harry Potter and the Chamber of Secrets"))
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title <> 'Harry Potter and the Philosopher''s Stone' OR title <> 'Harry Potter and the Chamber of Secrets';", &sql);
    /// // add                                    ^^^^^    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^    ^^^^^    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                   field                       value                       field                      value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_ne<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_ne(field, value);
        self
    }

    /// Add OR condition for field greater than value to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_lt("price", 100.to_string())
    ///     .or_where_gt("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price < 100 OR price > 300;", &sql);
    /// // add                                                          ^^^^^   ^^^
    /// // here                                                         field  value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_gt<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_gt(field, value);
        self
    }

    /// Add OR condition for field not less than value to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .or_where_lt("price", 100.to_string())
    ///     .or_where_ge("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price < 100 OR price >= 300;", &sql);
    /// // add                                           ^^^^^   ^^^    ^^^^^    ^^^
    /// // here                                          field  value   field   value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_ge<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_ge(field, value);
        self
    }

    /// Add OR condition for field less than value to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_lt("price", 100.to_string())
    ///     .or_where_lt("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price < 100 OR price < 300;", &sql);
    /// // add                                                          ^^^^^   ^^^
    /// // here                                                         field  value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_lt<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_lt(field, value);
        self
    }

    /// Add OR condition for field not greater than value to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .or_where_le("price", 100.to_string())
    ///     .or_where_le("price", 300.to_string())
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE price <= 100 OR price <= 300;", &sql);
    /// // add                                           ^^^^^    ^^^    ^^^^^    ^^^
    /// // here                                          field   value   field   value
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_le<S, T>(&mut self, field: S, value: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_le(field, value);
        self
    }

    /// Add OR LIKE condition to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_like("title", "%Alice's%")
    ///     .or_where_like("title", "%Philosopher's%")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '%Alice''s%' OR title LIKE '%Philosopher''s%';", &sql);
    /// // add                                    ^^^^^      ^^^^^^^^^^^^    ^^^^^      ^^^^^^^^^^^^^^^^^^
    /// // here                                   field          mask        field             mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_like<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_like(field, mask);
        self
    }

    /// Add OR LIKE condition to the last WHERE %condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_like_right("title", "Alice's")
    ///     .or_where_like_right("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '%Alice''s' OR title LIKE '%Philosopher''s';", &sql);
    /// // add                                    ^^^^^        ^^^^^^^^     ^^^^^        ^^^^^^^^^^^^^^
    /// // here                                   field          mask       field             mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_like_right<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_like_right(field, mask);
        self
    }

    /// Add OR LIKE condition to the last WHERE condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_like_left("title", "Alice's")
    ///     .or_where_like_left("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE 'Alice''s%' OR title LIKE 'Philosopher''s%';", &sql);
    /// // add                                    ^^^^^       ^^^^^^^^      ^^^^^       ^^^^^^^^^^^^^^  
    /// // here                                   field         mask        field            mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_like_left<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_like_left(field, mask);
        self
    }

    /// Add OR LIKE condition to the last WHERE %condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_like_any("title", "Alice's")
    ///     .or_where_like_any("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title LIKE '%Alice''s%' OR title LIKE '%Philosopher''s%';", &sql);
    /// // add                                    ^^^^^      ^^^^^^^^^^^^    ^^^^^      ^^^^^^^^^^^^^^^^^^
    /// // here                                   field          mask        field             mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_like_any<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_like_any(field, mask);
        self
    }

    /// Add OR NOT LIKE condition to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .and_where_not_like("title", "%Alice's%")
    ///     .or_where_not_like("title", "%Philosopher's%")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE title NOT LIKE '%Alice''s%' OR title NOT LIKE '%Philosopher''s%';", &sql);
    /// // add                                                                   ^^^^^          ^^^^^^^^^^^^^^^^^^
    /// // here                                                                  field                 mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_not_like<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_not_like(field, mask);
        self
    }

    /// Add OR NOT LIKE condition to the last WHERE %condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_not_like_right("title", "Alice's")
    ///     .or_where_not_like_right("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE '%Alice''s' OR title NOT LIKE '%Philosopher''s';", &sql);
    /// // add                                    ^^^^^            ^^^^^^^^     ^^^^^            ^^^^^^^^^^^^^^
    /// // here                                   field              mask       field                 mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_not_like_right<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_not_like_right(field, mask);
        self
    }

    /// Add OR NOT LIKE condition to the last WHERE condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_not_like_left("title", "Alice's")
    ///     .or_where_not_like_left("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE 'Alice''s%' OR title NOT LIKE 'Philosopher''s%';", &sql);
    /// // add                                    ^^^^^           ^^^^^^^^      ^^^^^           ^^^^^^^^^^^^^^  
    /// // here                                   field             mask        field                mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_not_like_left<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_not_like_left(field, mask);
        self
    }

    /// Add OR NOT LIKE condition to the last WHERE %condition%.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("price")
    ///     .or_where_not_like_any("title", "Alice's")
    ///     .or_where_not_like_any("title", "Philosopher's")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT price FROM books WHERE title NOT LIKE '%Alice''s%' OR title NOT LIKE '%Philosopher''s%';", &sql);
    /// // add                                    ^^^^^          ^^^^^^^^^^^^    ^^^^^          ^^^^^^^^^^^^^^^^^^
    /// // here                                   field              mask        field                 mask
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_not_like_any<S, T>(&mut self, field: S, mask: T) -> &mut Self
    where
        S: ToString,
        T: ToString,
    {
        self.builder.or_where_not_like_any(field, mask);
        self
    }

    /// Add OR IS NULL condition to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .and_where_eq("price", 0)
    ///     .or_where_is_null("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE price = 0 OR price IS NULL;", &sql);
    /// // add                                                 ^^^^^
    /// // here                                                field
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_is_null<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.or_where_is_null(field);
        self
    }

    /// Add OR IS NOT NULL condition to the last WHERE condition.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .or_where_is_not_null("title")
    ///     .or_where_is_not_null("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title FROM books WHERE title IS NOT NULL OR price IS NOT NULL;", &sql);
    /// // add                                    ^^^^^                ^^^^^
    /// // here                                   field                field
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_is_not_null<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.or_where_is_not_null(field);
        self
    }

    /// Union query with subquery.
    /// ORDER BY must be in the last subquery.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let append = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("price < 100")
    ///     .order_asc("title")
    ///     .query()?;
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_desc("price")
    ///     .union(&append)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' UNION SELECT title, price FROM books WHERE price < 100 ORDER BY title;", &sql);
    /// // add                                                                            ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                                                                                        query
    /// # Ok(())
    /// # }
    /// ```
    pub fn union<S: ToString>(&mut self, query: S) -> &mut Self {
        self.builder.union(query);
        self
    }

    /// Union query with all subquery.
    /// ORDER BY must be in the last subquery.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let append = Sqlite3Builder::select_values(&["'The Great Gatsby'", "124"])
    ///     .query_values()?;
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_desc("price")
    ///     .union_all(&append)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' UNION ALL SELECT 'The Great Gatsby', 124;", &sql);
    /// // add                                                                                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    /// // here                                                                                           query
    /// # Ok(())
    /// # }
    /// ```
    pub fn union_all<S: ToString>(&mut self, query: S) -> &mut Self {
        self.builder.union_all(query);
        self
    }

    /// Add ORDER BY.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_by("price", false)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price;", &sql);
    /// // add                                                                               ^^^^^
    /// // here                                                                              field
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_by<S: ToString>(&mut self, field: S, desc: bool) -> &mut Self {
        self.builder.order_by(field, desc);
        self
    }

    /// Add ORDER BY ASC.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_asc("title")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title;", &sql);
    /// // add                                                                               ^^^^^
    /// // here                                                                              field
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_asc<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.order_asc(field);
        self
    }

    /// Add ORDER BY DESC.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_desc("price")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC;", &sql);
    /// // add                                                                               ^^^^^
    /// // here                                                                              field
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_desc<S: ToString>(&mut self, field: S) -> &mut Self {
        self.builder.order_desc(field);
        self
    }

    /// Set LIMIT.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_desc("price")
    ///     .limit(10)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC LIMIT 10;", &sql);
    /// // add                                                                                                ^^
    /// // here                                                                                              limit
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit<S: ToString>(&mut self, limit: S) -> &mut Self {
        self.builder.limit(limit);
        self
    }

    /// Set OFFSET.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where_like_left("title", "Harry Potter")
    ///     .order_desc("price")
    ///     .limit(10)
    ///     .offset(100)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC LIMIT 10 OFFSET 100;", &sql);
    /// // add                                                                                                          ^^^
    /// // here                                                                                                        offset
    /// # Ok(())
    /// # }
    /// ```
    pub fn offset<S: ToString>(&mut self, offset: S) -> &mut Self {
        self.builder.offset(offset);
        self
    }

    /// Build complete SQL command.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books").sql()?;
    ///
    /// assert_eq!("SELECT * FROM books;", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn sql(&self) -> Result<String, Box<dyn Error>> {
        self.builder.sql()
    }

    /// Build subquery SQL command.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let cat = Sqlite3Builder::select_from("books")
    ///     .field("CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category")
    ///     .subquery()?;
    ///
    /// assert_eq!("(SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category FROM books)", &cat);
    ///
    /// let sql = Sqlite3Builder::select_from(&cat)
    ///     .field("category")
    ///     .field("COUNT(category) AS cnt")
    ///     .group_by("category")
    ///     .order_desc("cnt")
    ///     .order_asc("category")
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT category, COUNT(category) AS cnt FROM (SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category FROM books) GROUP BY category ORDER BY cnt DESC, category;", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn subquery(&self) -> Result<String, Box<dyn Error>> {
        self.builder.subquery()
    }

    /// Build named subquery SQL command.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let cat = Sqlite3Builder::select_from("books")
    ///     .field("CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END")
    ///     .subquery_as("category")?;
    ///
    /// assert_eq!("(SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END FROM books) AS category", &cat);
    /// // add                                                                                     ^^^^^^^^
    /// // here                                                                                      name
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .field(&cat)
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT title, price, (SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END FROM books) AS category FROM books;", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn subquery_as<S: ToString>(&self, name: S) -> Result<String, Box<dyn Error>> {
        self.builder.subquery_as(name)
    }

    /// SQL command generator for query or subquery.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let query = Sqlite3Builder::select_from("warehouse")
    ///     .field("title")
    ///     .field("preliminary_price * 2")
    ///     .query()?;
    ///
    /// assert_eq!("SELECT title, preliminary_price * 2 FROM warehouse", &query);
    ///
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .select(&query)
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) SELECT title, preliminary_price * 2 FROM warehouse;", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query(&self) -> Result<String, Box<dyn Error>> {
        self.builder.query()
    }

    /// SQL command generator for query or subquery without a table.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let values = Sqlite3Builder::select_values(&["10", &quote("100")])
    ///     .query_values()?;
    ///
    /// assert_eq!("SELECT 10, '100'", &values);
    /// # Ok(())
    /// # }
    /// ```
    pub fn query_values(&self) -> Result<String, Box<dyn Error>> {
        self.builder.query_values()
    }

    /// Convert sqlite3::Value to serde_json::Value
    fn s2j(src: &SValue) -> Result<JValue, Box<dyn Error>> {
        match src {
            SValue::Null => Ok(JValue::Null),
            SValue::Integer(val) => Ok(JValue::Number((*val).into())),
            SValue::String(val) => Ok(JValue::String(val.clone())),
            _ => Err("Unsupported type".into()),
        }
    }

    /// Execute request
    pub fn exec(&self, conn: &ConnPooled) -> Result<(), Box<dyn Error>> {
        let sql = self.builder.sql()?;
        debug!("Exec sql = {}", &sql);
        conn.execute(sql).map_err(|err| err.into())
    }

    /// Execute and return all data
    pub fn get(&self, conn: &ConnPooled) -> Result<Vec<Vec<JValue>>, Box<dyn Error>> {
        let sql = self.builder.sql()?;
        debug!("Get rows sql = {}", &sql);
        let mut result = Vec::new();
        let mut cursor = conn.prepare(sql)?.cursor();
        while let Some(row) = cursor.next()? {
            let jrow = row
                .iter()
                .map(|val| Self::s2j(&val).unwrap())
                .collect::<Vec<JValue>>();
            result.push(jrow);
        }
        Ok(result)
    }

    /// Execute and return first row
    pub fn get_row(&self, conn: &ConnPooled) -> Result<Vec<JValue>, Box<dyn Error>> {
        let sql = self.builder.sql()?;
        debug!("Get row sql = {}", &sql);
        let mut cursor = conn.prepare(sql)?.cursor();
        let first_row = if let Some(row) = cursor.next()? {
            row.iter()
                .map(|val| Self::s2j(&val).unwrap())
                .collect::<Vec<JValue>>()
        } else {
            Vec::new()
        };
        Ok(first_row)
    }

    /// Execute and return first value
    pub fn get_value(&self, conn: &ConnPooled) -> Result<JValue, Box<dyn Error>> {
        let sql = self.builder.sql()?;
        debug!("Get value sql = {}", &sql);
        let mut cursor = conn.prepare(sql)?.cursor();
        let first_value = if let Some(row) = cursor.next()? {
            Self::s2j(&row[0])?
        } else {
            return Err("No any value".into());
        };
        Ok(first_value)
    }

    /// Execute and return first integer value
    pub fn get_int(&self, conn: &ConnPooled) -> Result<i64, Box<dyn Error>> {
        Ok(self.get_value(&conn)?.as_i64().unwrap())
    }

    /// Execute and return first string value
    pub fn get_str(&self, conn: &ConnPooled) -> Result<String, Box<dyn Error>> {
        Ok(self.get_value(&conn)?.as_str().unwrap().to_string())
    }

    /// Get cursor for request
    pub fn get_cursor<'a>(&'a self, conn: &'a ConnPooled) -> Result<Cursor<'a>, Box<dyn Error>> {
        let sql = self.builder.sql()?;
        debug!("Get cursor sql = {}", &sql);
        let cursor = conn.prepare(sql)?.cursor();
        Ok(cursor)
    }
}

/// Escape string for SQL.
///
/// ```
/// extern crate sqlite3builder;
///
/// use sql_builder::esc;
///
/// let sql = esc("Hello, 'World'");
///
/// assert_eq!(&sql, "Hello, ''World''");
/// ```
pub fn esc(src: &str) -> String {
    SqlBuilderEsc(src)
}

/// Quote string for SQL.
///
/// ```
/// extern crate sqlite3builder;
///
/// use sql_builder::quote;
///
/// let sql = quote("Hello, 'World'");
///
/// assert_eq!(&sql, "'Hello, ''World'''");
/// ```
pub fn quote(src: &str) -> String {
    SqlBuilderQuote(src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esc() -> Result<(), Box<dyn Error>> {
        let sql = esc("Hello, 'World'");

        assert_eq!(&sql, "Hello, ''World''");

        Ok(())
    }

    #[test]
    fn test_quote() -> Result<(), Box<dyn Error>> {
        let sql = quote("Hello, 'World'");

        assert_eq!(&sql, "'Hello, ''World'''");

        Ok(())
    }

    #[test]
    fn test_select_only_values() -> Result<(), Box<dyn Error>> {
        let values = Sqlite3Builder::select_values(&["10", &quote("100")]).sql()?;

        assert_eq!("SELECT 10, '100';", &values);

        Ok(())
    }

    #[test]
    fn test_select_all_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books").sql()?;

        assert_eq!(&sql, "SELECT * FROM books;");

        Ok(())
    }

    #[test]
    fn test_show_all_prices() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .distinct()
            .field("price")
            .sql()?;

        assert_eq!(&sql, "SELECT DISTINCT price FROM books;");

        Ok(())
    }

    #[test]
    fn test_select_title_and_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .fields(&["title", "price"])
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books;");

        Ok(())
    }

    #[test]
    fn test_select_expensive_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("price > 100")
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE price > 100;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_gt("price", 200.to_string())
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE price > 200;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_ge("price", 300.to_string())
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE price >= 300;");

        Ok(())
    }

    #[test]
    fn test_select_price_for_harry_potter_and_phil_stone() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("price")
            .and_where_eq("title", quote("Harry Potter and the Philosopher's Stone"))
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT price FROM books WHERE title = 'Harry Potter and the Philosopher''s Stone';"
        );

        Ok(())
    }

    #[test]
    fn test_select_price_not_for_harry_potter_and_phil_stone() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("price")
            .and_where_ne("title", quote("Harry Potter and the Philosopher's Stone"))
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT price FROM books WHERE title <> 'Harry Potter and the Philosopher''s Stone';"
        );

        Ok(())
    }

    #[test]
    fn test_select_expensive_harry_potter() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("price > 100")
            .and_where_like_left("title", "Harry Potter")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');"
        );

        Ok(())
    }

    #[test]
    fn test_select_strange_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("price < 2")
            .or_where("price > 1000")
            .or_where_eq("title", quote("Harry Potter and the Philosopher's Stone"))
            .or_where_ne("price", 100)
            .or_where_like("title", "Alice's")
            .or_where_not_like_any("LOWER(title)", " the ")
            .or_where_is_null("title")
            .or_where_is_not_null("price")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE price < 2 OR price > 1000 OR title = 'Harry Potter and the Philosopher''s Stone' OR price <> 100 OR title LIKE 'Alice''s' OR LOWER(title) NOT LIKE '% the %' OR title IS NULL OR price IS NOT NULL;"
        );

        Ok(())
    }

    #[test]
    fn test_order_harry_potter_by_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_by("price", false)
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price;"
        );

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_desc("price")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC;"
        );

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_desc("price")
            .order_asc("title")
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC, title;");

        Ok(())
    }

    #[test]
    fn test_find_cheap_or_harry_potter() -> Result<(), Box<dyn Error>> {
        let append = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("price < 100")
            .order_asc("title")
            .query()?;

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_desc("price")
            .union(&append)
            .sql()?;

        assert_eq!(
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' UNION SELECT title, price FROM books WHERE price < 100 ORDER BY title;",
            &sql
        );

        let append =
            Sqlite3Builder::select_values(&["'The Great Gatsby'", "124"]).query_values()?;

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_desc("price")
            .union_all(&append)
            .sql()?;

        assert_eq!(
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' UNION ALL SELECT 'The Great Gatsby', 124;",
            &sql
        );

        Ok(())
    }

    #[test]
    fn test_select_first_3_harry_potter_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_asc("title")
            .limit(3)
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title LIMIT 3;");

        Ok(())
    }

    #[test]
    fn test_select_harry_potter_from_second_book() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_asc("title")
            .offset(2)
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title OFFSET 2;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where_like_left("title", "Harry Potter")
            .order_asc("title")
            .limit(3)
            .offset(2)
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title LIMIT 3 OFFSET 2;");

        Ok(())
    }

    #[test]
    fn test_find_books_not_about_alice() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .and_where_not_like_any("title", "Alice's")
            .sql()?;

        assert_eq!(
            "SELECT title FROM books WHERE title NOT LIKE '%Alice''s%';",
            &sql
        );

        Ok(())
    }

    #[test]
    fn test_books_without_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .and_where_is_null("price")
            .sql()?;

        assert_eq!(&sql, "SELECT title FROM books WHERE price IS NULL;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .and_where_is_not_null("price")
            .sql()?;

        assert_eq!(&sql, "SELECT title FROM books WHERE price IS NOT NULL;");

        Ok(())
    }

    #[test]
    fn test_group_books_by_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("price")
            .field("COUNT(price) AS cnt")
            .group_by("price")
            .order_desc("cnt")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT price, COUNT(price) AS cnt FROM books GROUP BY price ORDER BY cnt DESC;"
        );

        let sql = Sqlite3Builder::select_from("books")
            .field("price")
            .field("COUNT(price) AS cnt")
            .group_by("price")
            .having("price > 100")
            .order_desc("cnt")
            .sql()?;

        assert_eq!(&sql, "SELECT price, COUNT(price) AS cnt FROM books GROUP BY price HAVING price > 100 ORDER BY cnt DESC;");

        Ok(())
    }

    #[test]
    fn test_group_books_by_price_category() -> Result<(), Box<dyn Error>> {
        let cat = Sqlite3Builder::select_from("books")
            .field("CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category")
            .subquery()?;

        assert_eq!("(SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category FROM books)", &cat);

        let sql = Sqlite3Builder::select_from(&cat)
            .field("category")
            .field("COUNT(category) AS cnt")
            .group_by("category")
            .order_desc("cnt")
            .order_asc("category")
            .sql()?;

        assert_eq!("SELECT category, COUNT(category) AS cnt FROM (SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END AS category FROM books) GROUP BY category ORDER BY cnt DESC, category;", &sql);

        let cat = Sqlite3Builder::select_from("books")
            .field("CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END")
            .subquery_as("category")?;

        assert_eq!("(SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END FROM books) AS category", &cat);

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .field(&cat)
            .sql()?;

        assert_eq!("SELECT title, price, (SELECT CASE WHEN price < 100 THEN 'cheap' ELSE 'expensive' END FROM books) AS category FROM books;", &sql);

        Ok(())
    }

    #[test]
    fn test_grow_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::update_table("books")
            .set("price", "price + 10")
            .sql()?;

        assert_eq!(&sql, "UPDATE books SET price = price + 10;");

        let sql = Sqlite3Builder::update_table("books")
            .set("price", "price * 0.1")
            .and_where_like_left("title", "Harry Potter")
            .sql()?;

        assert_eq!(
            &sql,
            "UPDATE books SET price = price * 0.1 WHERE title LIKE 'Harry Potter%';"
        );

        Ok(())
    }

    #[test]
    fn test_add_new_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::insert_into("books")
            .field("title")
            .field("price")
            .values(&[quote("In Search of Lost Time"), 150.to_string()])
            .values(&["'Don Quixote', 200"])
            .sql()?;

        assert_eq!(&sql, "INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);");

        Ok(())
    }

    #[test]
    fn test_add_books_from_warehouse() -> Result<(), Box<dyn Error>> {
        let query = Sqlite3Builder::select_from("warehouse")
            .field("title")
            .field("preliminary_price * 2")
            .query()?;

        assert_eq!("SELECT title, preliminary_price * 2 FROM warehouse", &query);

        let sql = Sqlite3Builder::insert_into("books")
            .field("title")
            .field("price")
            .select(&query)
            .sql()?;

        assert_eq!(
            "INSERT INTO books (title, price) SELECT title, preliminary_price * 2 FROM warehouse;",
            &sql
        );

        Ok(())
    }

    #[test]
    fn test_sold_all_harry_potter() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::update_table("books")
            .set("price", 0)
            .set("title", "'[SOLD!]' || title")
            .and_where_like_left("title", "Harry Potter")
            .sql()?;

        assert_eq!(&sql, "UPDATE books SET price = 0, title = '[SOLD!]' || title WHERE title LIKE 'Harry Potter%';");

        Ok(())
    }

    #[test]
    fn test_mark_as_not_distr() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::update_table("books")
            .set_str("comment", "Don't distribute!")
            .and_where_le("price", "100")
            .sql()?;

        assert_eq!(
            "UPDATE books SET comment = 'Don''t distribute!' WHERE price <= 100;",
            &sql
        );

        Ok(())
    }

    #[test]
    fn test_remove_all_expensive_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::delete_from("books")
            .and_where("price > 100")
            .sql()?;

        assert_eq!(&sql, "DELETE FROM books WHERE price > 100;");

        Ok(())
    }

    #[test]
    fn test_count_books_in_shops() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books AS b")
            .field("b.title")
            .field("s.total")
            .left_outer()
            .join("shops AS s")
            .on("b.id = s.book")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT b.title, s.total FROM books AS b LEFT OUTER JOIN shops AS s ON b.id = s.book;"
        );

        Ok(())
    }
}
