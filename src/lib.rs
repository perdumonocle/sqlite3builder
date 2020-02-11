//! Simple SQL code generator. May be used with pooled Sqlite3 connection.
//!
//! # Examples:
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

#[macro_use]
extern crate log;
extern crate sql_builder;

use sql_builder::{SqlBuilder, esc as SqlBuilderEsc, quote as SqlBuilderQuote};
use serde_json::value::Value as JValue;
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
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn select_from(table: &str) -> Self {
        Self {
            builder: SqlBuilder::select_from(table),
        }
    }

    /// Create INSERT query.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .values(&[&quote("In Search of Lost Time"), "150"])
    ///     .values(&["'Don Quixote', 200"])
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn insert_into(table: &str) -> Self {
        Self {
            builder: SqlBuilder::insert_into(table),
        }
    }

    /// Create UPDATE query.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn update_table(table: &str) -> Self {
        Self {
            builder: SqlBuilder::update_table(table),
        }
    }

    /// Create DELETE query.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn delete_from(table: &str) -> Self {
        Self {
            builder: SqlBuilder::delete_from(table),
        }
    }

    /// Join with table.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books AS b")
    ///     .field("b.title")
    ///     .field("s.total")
    ///     .join("shops AS s", Some("LEFT OUTER"), Some("ON b.id = s.book"))
    ///     .sql()?;
    ///
    /// assert_eq!("SELECT b.title, s.total FROM books AS b LEFT OUTER JOIN shops AS s ON b.id = s.book;", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn join(
        &mut self,
        table: &str,
        operator: Option<&str>,
        constraint: Option<&str>,
    ) -> &mut Self {
        self.builder.join(table, operator, constraint);
        self
    }

    /// Set DISTINCT for fields.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn fields(&mut self, fields: &[&str]) -> &mut Self {
        self.builder.fields(fields);
        self
    }

    /// Replace fields.
    ///
    /// ```
    /// extern crate sql_builder;
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
    ///   let item = format!("LOWER(title) LIKE '%{}%'", filter.to_lowercase());
    ///   db.and_where(&item);
    /// }
    ///
    /// if let Some(price_min) = &req_data.price_min {
    ///   let item = format!("price >= {}", price_min);
    ///   db.and_where(&item);
    /// }
    ///
    /// if let Some(price_max) = &req_data.price_max {
    ///   let item = format!("price <= {}", price_max);
    ///   db.and_where(&item);
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
    pub fn set_fields(&mut self, fields: &[&str]) -> &mut Self {
        self.builder.set_fields(fields);
        self
    }

    /// Add field.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn field(&mut self, field: &str) -> &mut Self {
        self.builder.field(field);
        self
    }

    /// Replace fields with choosed one.
    ///
    /// ```
    /// extern crate sql_builder;
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
    ///   let item = format!("LOWER(title) LIKE '%{}%'", filter.to_lowercase());
    ///   db.and_where(&item);
    /// }
    ///
    /// if let Some(price_min) = &req_data.price_min {
    ///   let item = format!("price >= {}", price_min);
    ///   db.and_where(&item);
    /// }
    ///
    /// if let Some(price_max) = &req_data.price_max {
    ///   let item = format!("price <= {}", price_max);
    ///   db.and_where(&item);
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
    pub fn set_field(&mut self, field: &str) -> &mut Self {
        self.builder.set_field(field);
        self
    }

    /// Add SET part (for UPDATE).
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn set(&mut self, field: &str, value: &str) -> &mut Self {
        self.builder.set(field, value);
        self
    }

    /// Add VALUES part (for INSERT).
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .values(&[&quote("In Search of Lost Time"), "150"])
    ///     .values(&["'Don Quixote', 200"])
    ///     .sql()?;
    ///
    /// assert_eq!("INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);", &sql);
    /// # Ok(())
    /// # }
    /// ```
    pub fn values(&mut self, values: &[&str]) -> &mut Self {
        self.builder.values(values);
        self
    }

    /// Add SELECT part (for INSERT).
    ///
    /// ```
    /// extern crate sql_builder;
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
    pub fn select(&mut self, query: &str) -> &mut Self {
        self.builder.select(query);
        self
    }

    /// Add GROUP BY part.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::{Sqlite3Builder, quote};
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn group_by(&mut self, field: &str) -> &mut Self {
        self.builder.group_by(field);
        self
    }

    /// Add HAVING condition.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// # Ok(())
    /// # }
    /// ```
    pub fn having(&mut self, cond: &str) -> &mut Self {
        self.builder.having(cond);
        self
    }

    /// Add WHERE condition.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where(&mut self, cond: &str) -> &mut Self {
        self.builder.and_where(cond);
        self
    }

    /// Add WHERE condition for equal parts.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// assert_eq!(
    ///     "SELECT price FROM books WHERE title = 'Harry Potter and the Philosopher''s Stone';",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_eq(&mut self, field: &str, value: &str) -> &mut Self {
        self.builder.and_where_eq(field, value);
        self
    }

    /// Add WHERE condition for non-equal parts.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// assert_eq!(
    ///     "SELECT price FROM books WHERE title <> 'Harry Potter and the Philosopher''s Stone';",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn and_where_ne(&mut self, field: &str, value: &str) -> &mut Self {
        self.builder.and_where_ne(field, value);
        self
    }

    /// Add ORDER BY.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .order_by("price", false)
    ///     .sql()?;
    ///
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price;",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_by(&mut self, field: &str, desc: bool) -> &mut Self {
        self.builder.order_by(field, desc);
        self
    }

    /// Add ORDER BY ASC.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .order_asc("title")
    ///     .sql()?;
    ///
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title;",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_asc(&mut self, field: &str) -> &mut Self {
        self.builder.order_asc(field);
        self
    }

    /// Add ORDER BY DESC.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .order_desc("price")
    ///     .sql()?;
    ///
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC;",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_desc(&mut self, field: &str) -> &mut Self {
        self.builder.order_desc(field);
        self
    }

    /// Set LIMIT.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .order_desc("price")
    ///     .limit(10)
    ///     .sql()?;
    ///
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC LIMIT 10;",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.builder.limit(limit);
        self
    }

    /// Set OFFSET.
    ///
    /// ```
    /// extern crate sql_builder;
    ///
    /// # use std::error::Error;
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// # fn main() -> Result<(), Box<dyn Error>> {
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .order_desc("price")
    ///     .limit(10)
    ///     .offset(100)
    ///     .sql()?;
    ///
    /// assert_eq!(
    ///     "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC LIMIT 10 OFFSET 100;",
    ///     &sql
    /// );
    /// # Ok(())
    /// # }
    /// ```
    pub fn offset(&mut self, offset: usize) -> &mut Self {
        self.builder.offset(offset);
        self
    }

    /// Build complete SQL command.
    ///
    /// ```
    /// extern crate sql_builder;
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
    /// extern crate sql_builder;
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

    /// Build named subquery SQL command
    ///
    /// ```
    /// extern crate sql_builder;
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
    pub fn subquery_as(&self, name: &str) -> Result<String, Box<dyn Error>> {
        self.builder.subquery_as(name)
    }

    /// SQL command generator for query or subquery.
    ///
    /// ```
    /// extern crate sql_builder;
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

/// Escape string for SQL
pub fn esc(src: &str) -> String {
    SqlBuilderEsc(src)
}

/// Quote string for SQL
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

        Ok(())
    }

    #[test]
    fn test_select_price_for_harry_potter_and_phil_stone() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("price")
            .and_where_eq("title", &quote("Harry Potter and the Philosopher's Stone"))
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
            .and_where_ne("title", &quote("Harry Potter and the Philosopher's Stone"))
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
            .and_where("title LIKE 'Harry Potter%'")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');"
        );

        Ok(())
    }

    #[test]
    fn test_order_harry_potter_by_price() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("title LIKE 'Harry Potter%'")
            .order_by("price", false)
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price;"
        );

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("title LIKE 'Harry Potter%'")
            .order_desc("price")
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC;"
        );

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("title LIKE 'Harry Potter%'")
            .order_desc("price")
            .order_asc("title")
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY price DESC, title;");

        Ok(())
    }

    #[test]
    fn test_select_first_3_harry_potter_books() -> Result<(), Box<dyn Error>> {
        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("title LIKE 'Harry Potter%'")
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
            .and_where("title LIKE 'Harry Potter%'")
            .order_asc("title")
            .offset(2)
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title OFFSET 2;");

        let sql = Sqlite3Builder::select_from("books")
            .field("title")
            .field("price")
            .and_where("title LIKE 'Harry Potter%'")
            .order_asc("title")
            .limit(3)
            .offset(2)
            .sql()?;

        assert_eq!(&sql, "SELECT title, price FROM books WHERE title LIKE 'Harry Potter%' ORDER BY title LIMIT 3 OFFSET 2;");

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
            .and_where("title LIKE 'Harry Potter%'")
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
            .values(&[&quote("In Search of Lost Time"), "150"])
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
            .set("price", "0")
            .set("title", "'[SOLD!]' || title")
            .and_where("title LIKE 'Harry Potter%'")
            .sql()?;

        assert_eq!(&sql, "UPDATE books SET price = 0, title = '[SOLD!]' || title WHERE title LIKE 'Harry Potter%';");

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
            .join("shops AS s", Some("LEFT OUTER"), Some("ON b.id = s.book"))
            .sql()?;

        assert_eq!(
            &sql,
            "SELECT b.title, s.total FROM books AS b LEFT OUTER JOIN shops AS s ON b.id = s.book;"
        );

        Ok(())
    }
}
