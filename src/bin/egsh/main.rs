use std::env;
use std::io;
use std::io::Write;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use json::JsonValue;
use getopts;
use indicatif::ProgressBar;
use opensrf as osrf;
use evergreen as eg;
use eg::idl;
use eg::idldb;
use eg::idldb::IdlClassSearch;
use eg::db::DatabaseConnection;
use osrf::conf;
use osrf::client::Client;

const PROMPT: &str = "egsh#";
const DEFAULT_IDL_PATH: &str = "/openils/conf/fm_IDL.xml";

fn main() -> Result<(), String> {
    let mut opts = getopts::Options::new();
    let (conf, _) = osrf::init_with_options("service", &mut opts)?;

    let mut shell = Shell::setup(conf.into_shared(), &mut opts);
    shell.main_loop();

    Ok(())
}

struct SpinnerThread {
    stop: Arc<AtomicBool>,
}

impl SpinnerThread {

    fn start(&mut self) {
        let mut spinner = ProgressBar::new_spinner();

        loop {
            // Start with the sleep so the spinner only appears if the
            // request is actually taking enough time for a human to notice.
            thread::sleep(Duration::from_millis(50));

            if self.stop.load(Ordering::SeqCst) {
                // Main thread said to cut it out.
                spinner.finish();
                break;
            }

            spinner.tick();
        }
    }
}

struct Shell {
    idl: Arc<idl::Parser>,
    db: Option<Rc<RefCell<DatabaseConnection>>>,
    db_translator: Option<idldb::Translator>,
    config: Arc<conf::Config>,
    progress_flag: Arc<AtomicBool>,
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
            progress_flag: Arc::new(AtomicBool::new(false)),
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

    fn start_progress(&mut self) {
        let flag = self.progress_flag.clone();
        thread::spawn(|| SpinnerThread { stop: flag }.start());
    }

    fn stop_progress(&mut self) {
        self.progress_flag.store(true, Ordering::SeqCst);
    }

    fn main_loop(&mut self) {
        loop {
            print!("{PROMPT} ");
            io::stdout().flush().unwrap();
            if !self.read_one_command() {
                break;
            }
        }
    }

    /// Returns true if we should keep going.
    fn read_one_command(&mut self) -> bool {

        let mut user_input = String::new();

        io::stdin().read_line(&mut user_input);

        if user_input.len() == 0 {
            return true;
        }

        let user_input = user_input.trim();

        let parts: Vec<&str> = user_input.split(" ").collect();

        let command = parts[0].to_lowercase();

        match command.as_str() {
            "stop" | "quit" | "exit" => {
                return false;
            }
            "idl" => self.idl_query(&parts[1..]),
            _ => eprintln!("Unknown command: {command}"),
        }

        return true;
    }

    fn idl_query(&mut self, parts: &[&str]) {

        if parts.len() < 3 {
            eprintln!("'idl' command requires additional parameters: {parts:?}");
            return;
        }

        match parts[0] {
            "get" => self.idl_get(&parts[1..]),
            _ => {
                eprintln!("Could not parse idl query command: {parts:?}");
                return;
            }
        }
    }

    fn idl_get(&mut self, parts: &[&str]) {

        let mut translator = match self.db_translator.as_mut() {
            Some(t) => t,
            None => {
                eprintln!("no translator");
                return;
            }
        };

        if parts.len() < 2 {
            eprintln!("'idl get' command requires additional parameters: {parts:?}");
            return;
        }

        let idl_class = match self.idl.classes().get(parts[0]) {
            Some(c) => c,
            None => {
                eprintln!("No such IDL class: {}", parts[0]);
                return;
            }
        };

        let pkey_field = match idl_class.pkey() {
            Some(f) => f,
            None => {
                eprintln!(
                    "IDL class {} has no pkey value and cannot be queried",
                    idl_class.classname()
                );
                return;
            }
        };

        let mut filter = JsonValue::new_object();
        filter.insert(&pkey_field, parts[1]);

        let mut search = IdlClassSearch::new(parts[0]);
        search.set_filter(filter);

        for org in translator.idl_class_search(&search) { // ? todo
            println!("{org:?}");
        }
    }
}

