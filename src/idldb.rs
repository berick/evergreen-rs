///! Tools for translating between IDL objects and Database rows.
use log::{trace, error};
use json::JsonValue;
use super::db;
use super::idl;
use postgres as pg;
use std::rc::Rc;
use std::cell::{Ref, RefCell};

pub struct Translator {
    idl: Rc<RefCell<idl::Parser>>,
    db: Rc<RefCell<db::DatabaseConnection>>,
}

impl Translator {

    pub fn new(idl: Rc<RefCell<idl::Parser>>, db: Rc<RefCell<db::DatabaseConnection>>) -> Self {
        Translator {
            idl,
            db,
        }
    }

    pub fn search(&self, idlclass: &str, filter: &JsonValue) -> Result<Vec<JsonValue>, String> {

        let mut results: Vec<JsonValue> = Vec::new();
        let idl_parser = self.idl.borrow();

        let class = match idl_parser.classes().get(idlclass) {
            Some(c) => c,
            None => {
                return Err(format!("No such IDL class: {idlclass}"));
            }
        };

        let tablename = match class.tablename() {
            Some(t) => t,
            None => {
                return Err(format!(
                    "Cannot query an IDL class that has no tablename: {idlclass}"));
            }
        };

        let select = self.compile_class_select(&class);
        let filter = self.compile_class_filter(&class, filter)?;

        println!("compiled select: {select}");
        println!("compiled filter: {filter}");

        let query = format!("{select} FROM {tablename} {filter}");

        let query_res = self.db.borrow_mut().client().query(&query[..], &[]);

        if let Err(e) = query_res {
            return Err(format!("DB query failed: {e}"));
        }

        for row in query_res.unwrap() {
            results.push(self.row_to_idl(&class, &row)?);
        }

        Ok(results)
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

    pub fn compile_class_filter(&self,
        class: &idl::Class, filter: &JsonValue) -> Result<String, String> {

        if !filter.is_object() {
            return Err(format!(
                "Translator class filter must be an object: {}", filter.dump()));
        }

        let mut sql = String::from("WHERE");

        for (field, subq) in filter.entries() {
            trace!("compile_class_filter adding filter on field: {field}");

            sql += &format!(" {field}");

            match subq {
                JsonValue::Object(o) => {
                    // TODO
                },
                JsonValue::Array(a) => {
                    // TODO
                },
                JsonValue::Number(n) => {
                    sql += &format!(" = {n}");
                },
                JsonValue::String(s) => {
                    sql += &format!(" = QUOTE_LITERAL({})", s);
                },
                JsonValue::Short(s) => {
                    sql += &format!(" = QUOTE_LITERAL({})", s);
                }
                JsonValue::Boolean(b) => {
                    sql += &format!(" IS {b}");
                },
                JsonValue::Null => {
                    sql += " IS NULL";
                },
            }
        }

        Ok(sql)
    }

    pub fn row_to_idl(&self, class: &idl::Class, row: &pg::Row) -> Result<JsonValue, String> {

        let mut idx = 0;
        for (name, field) in class.fields() {
            let value: &str = row.get(idx);
            trace!("Row value {field}={value}");
            idx += 1;
        }

        Ok(JsonValue::Null)
    }
}



