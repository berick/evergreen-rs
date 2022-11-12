use std::env;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use getopts;
use indicatif;
use opensrf as osrf;
use evergreen as eg;
use eg::idl;
use eg::idldb;
use eg::db::DatabaseConnection;
use osrf::conf;
use osrf::client::Client;

const PROMPT: &str = "egsh# ";
const DEFAULT_IDL_PATH: &str = "/openils/conf/fm_IDL.xml";

fn main() -> Result<(), String> {
    let mut opts = getopts::Options::new();
    let (conf, _) = osrf::init_with_options("service", &mut opts)?;

    let mut shell = Shell::setup(conf.into_shared(), &mut opts);

    Ok(())
}

struct SpinnerThread {
}

struct Shell {
    idl: Arc<idl::Parser>,
    db: Option<Rc<RefCell<DatabaseConnection>>>,
    db_translator: Option<idldb::Translator>,
    config: Arc<conf::Config>,
}

impl Shell {
    fn setup(config: Arc<conf::Config>, opts: &mut getopts::Options) -> Shell {
        opts.optflag("", "with-database", "Open Direct Database Connection");
        opts.optopt("", "idl-file", "Path to IDL file", "IDL_PATH");

        DatabaseConnection::append_options(opts);

        let args: Vec<String> = env::args().collect();
        let params = opts.parse(&args[1..]).unwrap();

        // TODO pull the IDL path from opensrf.settings.
        let idl_file = params.opt_get_default("idl-file", DEFAULT_IDL_PATH.to_string()).unwrap();
        let idl = match idl::Parser::parse_file(&idl_file) {
            Ok(i) => i,
            Err(e) => panic!("Cannot parse IDL file: {} {}", e, idl_file),
        };

        let mut shell = Shell {
            config,
            idl,
            db: None,
            db_translator: None,
        };

        if params.opt_present("with-database") {
            shell.setup_db(&params);
        }

        shell
    }

    fn setup_db(&mut self, params: &getopts::Matches) {
        let mut db = DatabaseConnection::new_from_options(&params);

        if let Err(e) = db.connect() {
            panic!("Cannot connect to database: {}", e);
        }

        let db = db.into_shared();
        let translator = idldb::Translator::new(self.idl.clone(), db.clone());

        self.db = Some(db);
        self.db_translator = Some(translator);
    }
}

