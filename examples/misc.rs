use getopts;
use evergreen as eg;
use eg::db::DatabaseConnection;
use eg::idldb::Translator;
use std::env;
use json;

fn main() -> Result<(), String> {

    let args: Vec<String> = env::args().collect();
    let mut opts = getopts::Options::new();

    DatabaseConnection::append_options(&mut opts);

    let params = opts.parse(&args[1..]).unwrap();

    let db = DatabaseConnection::new_from_options(&params).to_shared();

    let idl = eg::idl::Parser::parse_file("/openils/conf/fm_IDL.xml").to_shared();

    let translator = Translator::new(idl, db);

    translator.search("aou", &json::object!{id: 1})?;

    Ok(())
}

