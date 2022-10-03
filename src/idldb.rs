use super::db;
use super::idl;
use json::JsonValue;
///! Tools for translating between IDL objects and Database rows.
use log::{debug, trace};
use postgres as pg;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum OrderByDir {
    Asc,
    Desc,
}

impl fmt::Display for OrderByDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match *self {
                OrderByDir::Asc => "ASC",
                _ => "DESC",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OrderBy {
    field: String,
    dir: OrderByDir,
}

impl OrderBy {
    pub fn new(field: &str, dir: OrderByDir) -> Self {
        OrderBy {
            dir,
            field: field.to_string(),
        }
    }
}

pub struct IdlClassSearch {
    pub classname: String,
    pub filter: Option<JsonValue>,
    pub order_by: Option<Vec<OrderBy>>,
}

pub struct Translator {
    idl: Arc<idl::Parser>,
    db: Rc<RefCell<db::DatabaseConnection>>,
}

impl Translator {
    pub fn new(idl: Arc<idl::Parser>, db: Rc<RefCell<db::DatabaseConnection>>) -> Self {
        Translator { idl, db }
    }

    pub fn idl(&self) -> &Arc<idl::Parser> {
        &self.idl
    }

    pub fn idl_class_search(&self, search: &IdlClassSearch) -> Result<Vec<JsonValue>, String> {
        let mut results: Vec<JsonValue> = Vec::new();
        let classname = &search.classname;

        let class = match self.idl().classes().get(classname) {
            Some(c) => c,
            None => {
                return Err(format!("No such IDL class: {classname}"));
            }
        };

        let tablename = match class.tablename() {
            Some(t) => t,
            None => {
                return Err(format!(
                    "Cannot query an IDL class that has no tablename: {classname}"
                ));
            }
        };

        let select = self.compile_class_select(&class);

        let mut query = format!("{select} FROM {tablename}");

        if let Some(filter) = &search.filter {
            query += &self.compile_class_filter(&class, filter)?;
        }

        if let Some(order) = &search.order_by {
            query += &self.compile_class_order_by(order);
        }

        debug!("search() executing query: {query}");

        let query_res = self.db.borrow_mut().client().query(&query[..], &[]);

        if let Err(e) = query_res {
            return Err(format!("DB query failed: {e}"));
        }

        for row in query_res.unwrap() {
            results.push(self.row_to_idl(&class, &row)?);
        }

        Ok(results)
    }

    pub fn compile_class_order_by(&self, order: &Vec<OrderBy>) -> String {
        let mut sql = String::new();
        let mut count = order.len();

        if count > 0 {
            sql += " ORDER BY";
            for order_by in order {
                sql += &format!(" {} {}", &order_by.field, &order_by.dir);
                count -= 1;
                if count > 0 {
                    sql += ",";
                }
            }
        }

        sql
    }

    pub fn compile_class_select(&self, class: &idl::Class) -> String {
        let mut sql = String::from("SELECT");

        for (name, field) in class.fields() {
            if !field.is_virtual() {
                sql += &format!(" {name},");
            }
        }

        String::from(&sql[0..sql.len() - 1]) // Trim final ","
    }

    pub fn json_literal_to_sql_value(&self, j: &JsonValue) -> Option<String> {
        match j {
            JsonValue::Number(n) => Some(n.to_string()),
            JsonValue::String(s) => Some(format!("'{}'", s.replace("'", "''"))),
            JsonValue::Short(s) => Some(format!("'{}'", s.replace("'", "''"))),
            JsonValue::Null => Some("NULL".to_string()),
            JsonValue::Boolean(b) => match b {
                true => Some("TRUE".to_string()),
                false => Some("FALSE".to_string()),
            },
            _ => None,
        }
    }

    /// Generate a WHERE clause from a JSON query object for an IDL class.
    pub fn compile_class_filter(
        &self,
        class: &idl::Class,
        filter: &JsonValue,
    ) -> Result<String, String> {
        if !filter.is_object() {
            return Err(format!(
                "Translator class filter must be an object: {}",
                filter.dump()
            ));
        }

        let mut sql = String::from(" WHERE");

        let mut first = true;
        for (field, subq) in filter.entries() {
            trace!("compile_class_filter adding filter on field: {field}");

            if class
                .fields()
                .iter()
                .filter(|(n, _)| n.eq(&field))
                .next()
                .is_none()
            {
                return Err(format!(
                    "Cannot query field '{field}' on class '{}'",
                    class.class()
                ));
            }

            if first {
                first = false;
            } else {
                sql += " AND";
            }

            sql += &format!(" {field}");

            if subq.is_string() || subq.is_number() {
                let literal = self.json_literal_to_sql_value(subq);
                sql += &format!(" = {}", literal.unwrap());
            } else if subq.is_boolean() || subq.is_null() {
                let literal = self.json_literal_to_sql_value(subq);
                sql += &format!(" IS {}", literal.unwrap());
            } else if subq.is_array() {
                sql += &self.compile_class_filter_array(&subq);
            } else {
                sql += &self.compile_class_filter_object(&subq)?;
            }
        }

        Ok(sql)
    }

    /// Turn an object-based subquery into part of the WHERE AND.
    pub fn compile_class_filter_object(&self, obj: &JsonValue) -> Result<String, String> {
        let mut sql = String::new();

        for (key, val) in obj.entries() {
            let value = match self.json_literal_to_sql_value(val) {
                Some(v) => v,
                None => {
                    return Err(format!("Arrays/Objects not supported here: {val:?}"));
                }
            };

            let operand = key.to_uppercase();

            match operand.as_str() {
                "IS" | "IS NOT" | "<" | "<=" | ">" | ">=" | "<>" | "!=" => {}
                _ => {
                    return Err(format!("Unsupported operand: {operand}"));
                }
            }

            sql += &format!(" {operand} {value}");
        }

        Ok(sql)
    }

    /// Turn an array-based subquery into part of the WHERE AND.
    pub fn compile_class_filter_array(&self, a: &JsonValue) -> String {
        let mut sql = String::from(" IN (");
        let mut first = true;

        for m in a.members() {
            if let Some(v) = self.json_literal_to_sql_value(m) {
                if first {
                    first = false;
                } else {
                    sql += ", "
                }
                sql += &format!("{v}");
            }
        }
        sql += ")";

        sql
    }

    /// Maps a PG row into an IDL-based JsonValue;
    pub fn row_to_idl(&self, class: &idl::Class, row: &pg::Row) -> Result<JsonValue, String> {
        let mut obj = JsonValue::new_object();
        obj[idl::CLASSNAME_KEY] = json::from(class.class());

        let mut index = 0;

        for (name, _) in class.fields().iter().filter(|(_, f)| !f.is_virtual()) {
            obj[name] = self.col_value_to_json_value(row, index)?;
            index += 1;
        }

        Ok(obj)
    }

    /// Translate a PG-typed row value into a JsonValue
    pub fn col_value_to_json_value(
        &self,
        row: &pg::Row,
        index: usize,
    ) -> Result<JsonValue, String> {
        let col_type = row.columns().get(index).map(|c| c.type_().name()).unwrap();

        match col_type {
            // JsonValue has From<Option<T>>
            "bool" => {
                let v: Option<bool> = row.get(index);
                Ok(json::from(v))
            }
            "varchar" | "char(n)" | "text" | "name" | "timestamp" | "timestamptz" => {
                let v: Option<String> = row.get(index);
                Ok(json::from(v))
            }
            "int2" | "smallserial" | "smallint" => {
                let v: Option<i16> = row.get(index);
                Ok(json::from(v))
            }
            "int" | "int4" | "serial" => {
                let v: Option<i32> = row.get(index);
                Ok(json::from(v))
            }
            "int8" | "bigserial" | "bigint" => {
                let v: Option<i64> = row.get(index);
                Ok(json::from(v))
            }
            "float4" | "real" => {
                let v: Option<f32> = row.get(index);
                Ok(json::from(v))
            }
            "float8" | "double precision" => {
                let v: Option<f64> = row.get(index);
                Ok(json::from(v))
            }
            _ => Err(format!("Unsupported column type: {col_type}")),
        }
    }
}
