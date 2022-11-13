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
use eg::idl::DataType;
use eg::idldb;
use eg::idldb::IdlClassSearch;
use eg::db::DatabaseConnection;
use osrf::conf;
use osrf::client::Client;
use rustyline;

const PROMPT: &str = "egsh# ";
const DEFAULT_IDL_PATH: &str = "/openils/conf/fm_IDL.xml";

fn main() -> Result<(), String> {
    let mut shell = Shell::setup();
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

    fn setup() -> Shell {
        let mut opts = getopts::Options::new();

        opts.optflag("", "with-database", "Open Direct Database Connection");
        opts.optopt("", "idl-file", "Path to IDL file", "IDL_PATH");

        DatabaseConnection::append_options(&mut opts);

        let conf = match osrf::init_with_options("service", &mut opts) {
            Ok((c, _)) => c,
            Err(e) => panic!("Cannot init to OpenSRF: {}", e),
        };

        let args: Vec<String> = env::args().collect();
        let params = opts.parse(&args[1..]).unwrap();

        // TODO pull the IDL path from opensrf.settings.
        let idl_file = params.opt_get_default("idl-file", DEFAULT_IDL_PATH.to_string()).unwrap();
        let idl = match idl::Parser::parse_file(&idl_file) {
            Ok(i) => i,
            Err(e) => panic!("Cannot parse IDL file: {} {}", e, idl_file),
        };

        let mut shell = Shell {
            config: conf.into_shared(),
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

        let config = rustyline::Config::builder()
            .history_ignore_space(true)
            .completion_type(rustyline::CompletionType::List)
            .edit_mode(rustyline::EditMode::Vi)
            .build();

        let mut readline = rustyline::Editor::with_config(config).unwrap();

        loop {
            match self.read_one_command(&mut readline) {
                Ok(line_op) => {
                    if let Some(line) = line_op {
                        readline.add_history_entry(&line);
                    }
                }
                Err(e) => eprintln!("Command failed: {e}"),
            }
        }
    }

    fn read_one_command(&mut self, readline: &mut rustyline::Editor<()>) -> Result<Option<String>, String> {

        let mut user_input = match readline.readline(PROMPT) {
            Ok(line) => line,
            Err(_) => return Ok(None)
        };

        let user_input = user_input.trim();

        if user_input.len() == 0 {
            return Ok(None);
        }

        let parts: Vec<&str> = user_input.split(" ").collect();

        let command = parts[0].to_lowercase();

        match command.as_str() {
            "stop" | "quit" | "exit" => std::process::exit(0x0),
            "idl" => {
                match self.idl_query(&parts[1..]) {
                    Ok(_) => return Ok(Some(user_input.to_string())),
                    Err(e) => return Err(e),
                }
            }
            _ => Err(format!("Unknown command: {command}")),
        }
    }

    fn idl_query(&mut self, parts: &[&str]) -> Result<(), String> {
        if parts.len() < 3 {
            return Err(format!("'idl' command requires additional parameters: {parts:?}"));
        }

        match parts[0] {
            "get" => self.idl_get(&parts[1..]),
            _ => return Err(format!("Could not parse idl query command: {parts:?}")),
        }
    }

    fn idl_get(&mut self, parts: &[&str]) -> Result<(), String> {

        if parts.len() < 2 {
            return Err(format!("'idl get' command requires additional parameters: {parts:?}"));
        }

        let mut translator = match self.db_translator.as_mut() {
            Some(t) => t,
            None => return Err(format!("Database connection required")),
        };

        let idl_class = match self.idl.classes().get(parts[0]) {
            Some(c) => c,
            None => return Err(format!("No such IDL class: {}", parts[0])),
        };

        let pkey_field = match idl_class.pkey() {
            Some(f) => f,
            None => {
                return Err(format!(
                    "IDL class {} has no pkey value and cannot be queried",
                    idl_class.classname()
                ));
            }
        };

        let idl_field = match idl_class.fields().get(pkey_field) {
            Some(f) => f,
            None => return Err(format!(
                "Field {pkey_field} is listed as pkey, but is not listed as a field"))
        };

        let pkey_arg = parts[1];
        let mut filter = JsonValue::new_object();

        if idl_field.datatype().is_numeric() {
            let num = match pkey_arg.parse::<f64>() {
                Ok(n) => n,
                Err(e) => return Err(format!(
                    "Pkey is numeric, but filter value provided is not: {pkey_arg:?}"))
            };

            filter.insert(&pkey_field, json::from(num)).unwrap();
        } else {

            filter.insert(&pkey_field, json::from(pkey_arg)).unwrap();
        }

        let mut search = IdlClassSearch::new(parts[0]);
        search.set_filter(filter);

        let mut fields: Vec<&String> = Vec::new();
        for (name, field) in idl_class.fields().into_iter() {
            if !field.is_virtual() {
                fields.push(name);
            }
        }
        fields.sort();

        if let Some(org) = translator.idl_class_search(&search)?.first() {
            for field in fields {
                println!("{field}\t{}", org[field]);
            }
        }

        Ok(())
    }
}

