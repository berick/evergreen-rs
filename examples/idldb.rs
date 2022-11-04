use eg::db::DatabaseConnection;
use eg::idl;
use eg::idldb::{IdlClassSearch, OrderBy, OrderByDir, Pager, Translator};
use evergreen as eg;
use getopts;
use opensrf::Config;
use opensrf::Logger;
use std::env;

fn main() -> Result<(), String> {
    let mut conf = Config::from_file("conf/opensrf.yml")?;
    let con = conf.set_primary_connection("service", "private.localhost")?;

    let ct = con.connection_type();
    Logger::new(ct.log_level(), ct.log_facility())
        .init()
        .unwrap();

    let args: Vec<String> = env::args().collect();
    let mut opts = getopts::Options::new();

    DatabaseConnection::append_options(&mut opts);

    let params = opts.parse(&args[1..]).unwrap();

    let mut db = DatabaseConnection::new_from_options(&params);
    db.connect()?;
    let db = db.to_shared();

    let idl = idl::Parser::parse_file("/openils/conf/fm_IDL.xml")?;

    let translator = Translator::new(idl.clone(), db.clone());

    // Give me all rows
    let mut search = IdlClassSearch::new("aou");

    for org in translator.idl_class_search(&search)? {
        println!("org: {} {}\n", org["id"], org["shortname"]);
    }

    search.set_filter(json::object! {id: 1, name: "CONS", opac_visible: false });

    for org in translator.idl_class_search(&search)? {
        println!("org: {} {}\n", org["id"], org["shortname"]);
    }

    search.set_filter(json::object! {id: json::object! {">": 1}, ou_type: [1, 2, 3]});

    for org in translator.idl_class_search(&search)? {
        println!("org: {} {}\n", org["id"], org["shortname"]);
    }

    search.set_order_by(vec![OrderBy::new("name", OrderByDir::Asc)]);

    for org in translator.idl_class_search(&search)? {
        println!("org: {} {}\n", org["id"], org["shortname"]);
    }

    search.set_pager(Pager::new(10, 0));

    for org in translator.idl_class_search(&search)? {
        println!("org: {} {}\n", org["id"], org["shortname"]);
    }

    Ok(())
}
