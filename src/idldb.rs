///! Tools for translating between IDL objects and Database rows.
use log::error;
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

        let results: Vec<JsonValue> = Vec::new();
        let idl_parser = self.idl.borrow();

        let class = match idl_parser.classes().get(idlclass) {
            Some(c) => c,
            None => {
                return Err(format!("No such IDL class: {idlclass}"));
            }
        };

        let select = self.compile_class_select(&class);

        println!("compiled select: {select}");

        /*
        let query_res = self.db.borrow().client().query(&query[..], &[]);

        if let Err(e) = query_res {
            return Err(format!("DB query failed: {e}"));
        }

        for row in query_res.unwrap() {
            results.push(self.row_to_idl(idlclass, &row)?);
        }
        */

        Ok(results)
    }

    pub fn compile_class_select(&self, class: &idl::Class) -> String {

        /*
        if !query.is_object() {
            return Err(format!("Translator query must be an object: {query:?}"));
        }
        */

        let mut sql = String::from("SELECT");

        for (name, field) in class.fields() {
            if !field.is_virtual() {
                sql += &format!(" {name},");
            }
        }

        String::from(&sql[0..sql.len() - 1]) // Trim final ","
    }

    pub fn row_to_idl(&self, idlclass: &str, row: &pg::Row) -> Result<JsonValue, String> {
        Ok(JsonValue::Null)
    }
}



