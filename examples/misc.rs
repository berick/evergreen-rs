use getopts;
use evergreen as eg;
use eg::idl;
use eg::db::DatabaseConnection;
use eg::idldb::Translator;
use std::env;
use json::JsonValue;

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
    let idl = idl.to_shared();

    let translator = Translator::new(idl.clone(), db.clone());

    let filter = json::object!{id: 1, ou_type: [1, 2, 3]};

    let results = translator.search("aou", &filter)?;

    for org in results {
        println!("org = {}\n", org.dump());
    }

    let filter = json::object!{parent_ou: JsonValue::Null};
    let results = translator.search("aou", &filter)?;

    for org in results {
        println!("org = {}\n", org.dump());
    }

    let filter = json::object!{id: json::object!{">": 1}};
    for org in translator.search("aou", &filter)? {
        println!("org = {}\n", org.dump());
    }

    Ok(())
}

