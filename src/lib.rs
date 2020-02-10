//! Simple SQL code generator. May be used with pooled Sqlite3 connection.
//!
//! Examples:
//!
//! ```
//! extern crate sqlite3builder;
//!
//! use sqlite3builder::Sqlite3Builder;
//!
//! let sql = Sqlite3Builder::select_from("COMPANY")
//!     .field("id")
//!     .field("name")
//!     .and_where("SALARY > 25000")
//!     .sql();
//!
//! assert_eq!(Ok("SELECT id, name FROM COMPANY WHERE SALARY > 25000;"), sql.as_ref());
//! ```

#[macro_use]
extern crate log;

use serde_json::value::Value as JValue;
use sqlite3::Cursor;
use sqlite3::Value as SValue;
use std::error::Error;

/// Pooled Sqlite3 connection
type ConnPooled = r2d2::PooledConnection<r2d2_sqlite3::SqliteConnectionManager>;

/// Builder stored info
pub struct Sqlite3Builder {
    action: Action,
    table: String,
    joins: Vec<String>,
    distinct: bool,
    fields: Vec<String>,
    sets: Vec<String>,
    values: Vec<String>,
    group_by: Vec<String>,
    having: Option<String>,
    wheres: Vec<String>,
    order_by: Vec<String>,
    limit: Option<usize>,
    offset: Option<usize>,
}

/// SQL query main action
enum Action {
    SelectFrom,
    UpdateTable,
    InsertInto,
    DeleteFrom,
}

impl Sqlite3Builder {
    /// Default constructor for struct
    fn default() -> Self {
        Self {
            action: Action::SelectFrom,
            table: String::new(),
            joins: Vec::new(),
            distinct: false,
            fields: Vec::new(),
            sets: Vec::new(),
            values: Vec::new(),
            group_by: Vec::new(),
            having: None,
            wheres: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Create SELECT request.
    /// You may specify comma separted list of tables.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .field("title")
    ///     .field("price")
    ///     .and_where("price > 100")
    ///     .and_where("title LIKE 'Harry Potter%'")
    ///     .sql();
    ///
    /// assert_eq!(Ok("SELECT title, price FROM books WHERE (price > 100) AND (title LIKE 'Harry Potter%');"), &sql.as_ref());
    /// ```
    pub fn select_from(table: &str) -> Self {
        Self {
            table: table.to_string(),
            ..Self::default()
        }
    }

    /// Create INSERT request.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::insert_into("books")
    ///     .field("title")
    ///     .field("price")
    ///     .values(&[&quote("In Search of Lost Time"), "150"])
    ///     .values(&["'Don Quixote', 200"])
    ///     .sql();
    ///
    /// assert_eq!(Ok("INSERT INTO books (title, price) VALUES ('In Search of Lost Time', 150), ('Don Quixote', 200);"), &sql.as_ref());
    /// ```
    pub fn insert_into(table: &str) -> Self {
        Self {
            action: Action::InsertInto,
            table: table.to_string(),
            ..Self::default()
        }
    }

    /// Create UPDATE request.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::update_table("books")
    ///     .set("price", "price + 10")
    ///     .sql();
    ///
    /// assert_eq!(Ok("UPDATE books SET price = price + 10;"), &sql.as_ref());
    /// ```
    pub fn update_table(table: &str) -> Self {
        Self {
            action: Action::UpdateTable,
            table: table.to_string(),
            ..Self::default()
        }
    }

    /// Create DELETE request.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::delete_from("books")
    ///     .and_where("price > 100")
    ///     .sql();
    ///
    /// assert_eq!(Ok("DELETE FROM books WHERE price > 100;"), &sql.as_ref());
    /// ```
    pub fn delete_from(table: &str) -> Self {
        Self {
            action: Action::DeleteFrom,
            table: table.to_string(),
            ..Self::default()
        }
    }

    /// Join with table.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::select_from("books AS b")
    ///     .field("b.title")
    ///     .field("s.total")
    ///     .join("shops AS s", Some("LEFT OUTER"), Some("ON b.id = s.book"))
    ///     .sql();
    ///
    /// assert_eq!(Ok("SELECT b.title, s.total FROM books AS b LEFT OUTER JOIN shops AS s ON b.id = s.book;"), &sql.as_ref());
    /// ```
    pub fn join(
        &mut self,
        table: &str,
        operator: Option<&str>,
        constraint: Option<&str>,
    ) -> &mut Self {
        let operator = if let Some(oper) = operator {
            format!("{} JOIN ", &oper)
        } else {
            String::new()
        };

        let constraint = if let Some(cons) = constraint {
            format!(" {}", &cons)
        } else {
            String::new()
        };

        let text = format!("{}{}{}", &operator, &table, &constraint);

        self.joins.push(text);
        self
    }

    /// Set DISTINCT for fields.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .distinct()
    ///     .field("price")
    ///     .sql();
    ///
    /// assert_eq!(Ok("SELECT DISTINCT price FROM books;"), &sql.as_ref());
    /// ```
    pub fn distinct(&mut self) -> &mut Self {
        self.distinct = true;
        self
    }

    /// Add fields.
    ///
    /// ```
    /// extern crate sqlite3builder;
    ///
    /// use sqlite3builder::Sqlite3Builder;
    ///
    /// let sql = Sqlite3Builder::select_from("books")
    ///     .fields(&["title", "price"])
    ///     .sql();
    ///
    /// assert_eq!(Ok("SELECT title, price FROM books;"), &sql.as_ref());
    /// ```
    pub fn fields(&mut self, fields: &[&str]) -> &mut Self {
        let mut fields = fields
            .iter()
            .map(|f| (*f).to_string())
            .collect::<Vec<String>>();
        self.fields.append(&mut fields);
        self
    }

    // TODO documentaion

    /// Replace fields
    pub fn set_fields(&mut self, fields: &[&str]) -> &mut Self {
        let fields = fields
            .iter()
            .map(|f| (*f).to_string())
            .collect::<Vec<String>>();
        self.fields = fields;
        self
    }

    /// Add field
    pub fn field(&mut self, field: &str) -> &mut Self {
        self.fields.push(field.to_string());
        self
    }

    /// Replace fields with choosed one
    pub fn set_field(&mut self, field: &str) -> &mut Self {
        self.fields = vec![field.to_string()];
        self
    }

    /// Add SET part (for UPDATE)
    pub fn set(&mut self, field: &str, value: &str) -> &mut Self {
        let expr = format!("{} = {}", &field, &value);
        self.sets.push(expr);
        self
    }

    /// Add VALUES part (for INSERT)
    pub fn values(&mut self, values: &[&str]) -> &mut Self {
        let values: Vec<String> = values
            .iter()
            .map(|v| (*v).to_string())
            .collect::<Vec<String>>();
        let values = format!("({})", values.join(", "));
        self.values.push(values);
        self
    }

    /// Add GROUP BY part
    pub fn group_by(&mut self, field: &str) -> &mut Self {
        self.group_by.push(field.to_string());
        self
    }

    /// Add HAVING condition
    pub fn having(&mut self, cond: &str) -> &mut Self {
        self.having = Some(cond.to_string());
        self
    }

    /// Add WHERE condition
    pub fn and_where(&mut self, cond: &str) -> &mut Self {
        self.wheres.push(cond.to_string());
        self
    }

    /// Add WHERE condition for equal parts
    pub fn and_where_eq(&mut self, field: &str, value: &str) -> &mut Self {
        let cond = format!("{} = {}", &field, &value);
        self.and_where(&cond)
    }

    /// Add ORDER BY
    pub fn order_by(&mut self, field: &str, desc: bool) -> &mut Self {
        let order = if desc {
            format!("{} DESC", &field)
        } else {
            field.to_string()
        };
        self.order_by.push(order);
        self
    }

    /// Add ORDER BY ASC
    pub fn order_asc(&mut self, field: &str) -> &mut Self {
        self.order_by(&field, false)
    }

    /// Add ORDER BY DESC
    pub fn order_desc(&mut self, field: &str) -> &mut Self {
        self.order_by(&field, true)
    }

    /// Set LIMIT
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    /// Set OFFSET
    pub fn offset(&mut self, offset: usize) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    /// Build complete SQL command
    pub fn sql(&self) -> Result<String, Box<dyn Error>> {
        match self.action {
            Action::SelectFrom => self.sql_select(),
            Action::UpdateTable => self.sql_update(),
            Action::InsertInto => self.sql_insert(),
            Action::DeleteFrom => self.sql_delete(),
        }
    }

    /// Build complete SQL command for SELECT action
    fn sql_select(&self) -> Result<String, Box<dyn Error>> {
        let mut text = self.query()?;
        text.push(';');
        Ok(text)
    }

    /// Build subquery SQL command
    pub fn subquery(&self) -> Result<String, Box<dyn Error>> {
        let text = self.query()?;
        let text = format!("({})", &text);
        Ok(text)
    }

    /// Build named subquery SQL command
    pub fn subquery_as(&self, name: &str) -> Result<String, Box<dyn Error>> {
        let text = self.query()?;
        let text = format!("({}) AS {}", &text, &name);
        Ok(text)
    }

    /// SQL command generator for query or subquery
    fn query(&self) -> Result<String, Box<dyn Error>> {
        // Checks
        if self.table.is_empty() {
            return Err("No table name".into());
        }

        // Distinct results
        let distinct = if self.distinct { " DISTINCT" } else { "" };

        // Make fields
        let fields = if self.fields.is_empty() {
            "*".to_string()
        } else {
            self.fields.join(", ")
        };

        // Make JOIN parts
        let joins = if self.joins.is_empty() {
            String::new()
        } else {
            format!(" {}", self.joins.join(" "))
        };

        // Make GROUP BY part
        let group_by = if self.group_by.is_empty() {
            String::new()
        } else {
            let having = if let Some(having) = &self.having {
                format!(" HAVING {}", having)
            } else {
                String::new()
            };
            format!(" GROUP BY {}{}", self.group_by.join(", "), having)
        };

        // Make WHERE part
        let wheres = Sqlite3Builder::make_wheres(&self.wheres);

        // Make ORDER BY part
        let order_by = if self.order_by.is_empty() {
            String::new()
        } else {
            format!(" ORDER BY {}", self.order_by.join(", "))
        };

        // Make LIMIT part
        let limit = match self.limit {
            Some(limit) => format!(" LIMIT {}", limit),
            None => String::new(),
        };

        // Make OFFSET part
        let offset = match self.offset {
            Some(offset) => format!(" OFFSET {}", offset),
            None => String::new(),
        };

        // Make SQL
        let sql = format!("SELECT{distinct} {fields} FROM {table}{joins}{group_by}{wheres}{order_by}{limit}{offset}",
            distinct = distinct,
            fields = fields,
            table = &self.table,
            joins = joins,
            group_by = group_by,
            wheres = wheres,
            order_by = order_by,
            limit = limit,
            offset = offset,
        );
        Ok(sql)
    }

    /// Build SQL command for INSERT action
    fn sql_insert(&self) -> Result<String, Box<dyn Error>> {
        // Checks
        if self.table.is_empty() {
            return Err("No table name".into());
        }
        if self.values.is_empty() {
            return Err("No set fields".into());
        }

        // Make SET part
        let fields = self.fields.join(", ");

        // Make VALUES part
        let values = self.values.join(", ");

        // Make SQL
        let sql = format!(
            "INSERT INTO {table} ({fields}) VALUES {values};",
            table = &self.table,
            fields = fields,
            values = values,
        );
        Ok(sql)
    }

    /// Build SQL command for UPDATE action
    fn sql_update(&self) -> Result<String, Box<dyn Error>> {
        // Checks
        if self.table.is_empty() {
            return Err("No table name".into());
        }
        if self.sets.is_empty() {
            return Err("No set fields".into());
        }

        // Make SET part
        let sets = self.sets.join(", ");

        // Make WHERE part
        let wheres = Sqlite3Builder::make_wheres(&self.wheres);

        // Make SQL
        let sql = format!(
            "UPDATE {table} SET {sets}{wheres};",
            table = &self.table,
            sets = sets,
            wheres = wheres,
        );
        Ok(sql)
    }

    /// Build SQL command for DELETE action
    fn sql_delete(&self) -> Result<String, Box<dyn Error>> {
        // Checks
        if self.table.is_empty() {
            return Err("No table name".into());
        }

        // Make WHERE part
        let wheres = Sqlite3Builder::make_wheres(&self.wheres);

        // Make SQL
        let sql = format!(
            "DELETE FROM {table}{wheres};",
            table = &self.table,
            wheres = wheres,
        );
        Ok(sql)
    }

    /// Make WHERE part
    fn make_wheres(wheres: &[String]) -> String {
        match wheres.len() {
            0 => String::new(),
            1 => {
                let wheres = wheres[0].to_string();
                format!(" WHERE {}", wheres)
            }
            _ => {
                let wheres: Vec<String> = wheres.iter().map(|w| format!("({})", w)).collect();
                format!(" WHERE {}", wheres.join(" AND "))
            }
        }
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
        let sql = self.sql()?;
        debug!("Exec sql = {}", &sql);
        conn.execute(sql).map_err(|err| err.into())
    }

    /// Execute and return all data
    pub fn get(&self, conn: &ConnPooled) -> Result<Vec<Vec<JValue>>, Box<dyn Error>> {
        let sql = self.sql()?;
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
        let sql = self.sql()?;
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
        let sql = self.sql()?;
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
        let sql = self.sql()?;
        debug!("Get cursor sql = {}", &sql);
        let cursor = conn.prepare(sql)?.cursor();
        Ok(cursor)
    }
}

/// Escape string for SQL
pub fn esc(src: &str) -> String {
    src.replace("'", "''")
}

/// Quote string for SQL
pub fn quote(src: &str) -> String {
    format!("'{}'", esc(src))
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
