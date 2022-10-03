use eg::db::DatabaseConnection;
use eg::idl;
use eg::idldb::{IdlClassSearch, OrderBy, OrderByDir, Translator};
use evergreen as eg;
use getopts;
use json::JsonValue;
use std::env;

fn main() -> Result<(), String> {
    env_logger::init();

    let args: Vec<String> = env::args().collect();
    let mut opts = getopts::Options::new();

    DatabaseConnection::append_options(&mut opts);

    let params = opts.parse(&args[1..]).unwrap();

    let mut db = DatabaseConnection::new_from_options(&params);
    db.connect()?;
    let db = db.to_shared();

    let idl = idl::Parser::parse_file("/openils/conf/fm_IDL.xml")?;

    let translator = Translator::new(idl.clone(), db.clone());

    let mut search = IdlClassSearch {
        classname: String::from("aou"),
        filter: Some(json::object! {id: 1, ou_type: [1, 2, 3]}),
        order_by: None,
    };

    let results = translator.idl_class_search(&search)?;

    for org in results {
        println!("org 1: {}\n", org.dump());
    }

    search.filter = Some(json::object! {parent_ou: JsonValue::Null});
    let results = translator.idl_class_search(&search)?;

    for org in results {
        println!("org 2: {}\n", org.dump());
    }

    search.filter = Some(json::object! {id: json::object!{">": 1}});
    search.order_by = Some(vec![OrderBy::new("name", OrderByDir::Asc)]);

    for org in translator.idl_class_search(&search)? {
        println!("org 3: {}\n", org.dump());
    }

    search.filter = None; // retrieve-all
    for org in translator.idl_class_search(&search)? {
        println!("org 4: {}\n", org.dump());
    }

    Ok(())
}
