use std::env;
use std::io;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use rustyline;
use rustyline::CompletionType;
use getopts;
use indicatif::ProgressBar;

use opensrf as osrf;
use evergreen as eg;
use eg::idl;
use eg::idldb;
use eg::db::DatabaseConnection;
use osrf::conf;
use osrf::client::Client;

//const PROMPT: &str = "egsh# ";
const PROMPT: &str = "\x1b[1;32megsh# \x1b[0m";
const DEFAULT_IDL_PATH: &str = "/openils/conf/fm_IDL.xml";
const HISTORY_FILE: &str = ".egsh_history";
const SEPARATOR: &str = "----------------------------------------------";
const DEFAULT_REQUEST_TIMEOUT: i32 = 120;
const DEFAULT_JSON_PRINT_DEPTH: u16 = 2;

const HELP_TEXT: &str = r#"
Options

  * Standard OpenSRF Options
    --localhost
    --domain
    --osrf-config

  * Additional Options
    --with-database
      * Connect directly to an Evergreen database.

Commands

  idl get <classname> <pkey-value>
    Retrieve and IDL-classed object by primary key.

  db sleep <seconds>
    Runs PG_SLEEP(<seconds>).  Mostly for debugging.

  router <node_name> <command> [<router_class>]
    Sends <command> to the router at <node_name> and reports the result.
    Specify "_" as the <node_name> to send the request to the router
    on the same node as the primary connection node for egsh.

  req     <service> <api_name> [<param>, <param>, ...]
  request <service> <api_name> [<param>, <param>, ...]
    Send an API request.

  help

"#;


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
        let spinner = ProgressBar::new_spinner();

        loop {
            // Start with the sleep so the spinner only appears if the
            // request is actually taking enough time for a human to notice.
            thread::sleep(Duration::from_millis(50));

            if self.stop.load(Ordering::SeqCst) {
                // Main thread said to cut -> it -> out.
                spinner.finish();
                break;
            }

            spinner.tick();
        }
    }
}

struct SpinnerThreadController {
    progress_flag: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
    active: bool,
}

impl SpinnerThreadController {

    /// Show the progress spinner
    fn show(&mut self) {
        self.active = true;
        let flag = self.progress_flag.clone();
        self.join_handle = Some(
            thread::spawn(|| SpinnerThread { stop: flag }.start())
        );
    }

    /// Hide the progress spinner
    fn hide(&mut self) {
        if !self.active { return; }

        self.progress_flag.store(true, Ordering::SeqCst);
        if let Some(h) = self.join_handle.take() {
            h.join().ok();
        }
        self.progress_flag.store(false, Ordering::SeqCst);
        self.active = false;
    }
}

/// Collection of context data, etc. for our shell.
struct Shell {
    idl: Arc<idl::Parser>,
    db: Option<Rc<RefCell<DatabaseConnection>>>,
    db_translator: Option<idldb::Translator>,
    client: Client,
    config: Arc<conf::Config>,
    history_file: Option<String>,
    spinner: SpinnerThreadController,
    json_print_depth: u16,
}

impl Shell {

    /// Handle command line options, OpenSRF init, build the Shell struct.
    fn setup() -> Shell {

        let spinner = SpinnerThreadController {
            progress_flag: Arc::new(AtomicBool::new(false)),
            join_handle: None,
            active: false
        };

        let mut opts = getopts::Options::new();

        opts.optflag("", "with-database", "Open Direct Database Connection");
        opts.optopt("", "idl-file", "Path to IDL file", "IDL_PATH");

        // We don't know if the user passed --with-database until after
        // we parse the command line options.  Append the DB options
        // in case we need them.
        DatabaseConnection::append_options(&mut opts);

        let conf = match osrf::init_with_options("service", &mut opts) {
            Ok((c, _)) => c,
            Err(e) => panic!("Cannot init to OpenSRF: {}", e),
        };

        let conf = conf.into_shared();

        let args: Vec<String> = env::args().collect();
        let params = opts.parse(&args[1..]).unwrap();

        // TODO pull the IDL path from opensrf.settings, while allowing
        // for override for testing purposes.
        let idl_file = params.opt_get_default(
            "idl-file", DEFAULT_IDL_PATH.to_string()).unwrap();

        let idl = match idl::Parser::parse_file(&idl_file) {
            Ok(i) => i,
            Err(e) => panic!("Cannot parse IDL file: {} {}", e, idl_file),
        };

        let client = match Client::connect(conf.clone()) {
            Ok(c) => c,
            Err(e) => panic!("Cannot connect to OpenSRF: {}", e),
        };

        client.set_serializer(idl::Parser::as_serializer(&idl));

        let mut shell = Shell {
            config: conf,
            client,
            idl,
            spinner,
            db: None,
            db_translator: None,
            history_file: None,
            json_print_depth: DEFAULT_JSON_PRINT_DEPTH,
        };

        if params.opt_present("with-database") {
            shell.setup_db(&params);
        }

        shell
    }

    /// Connect directly to the specified database.
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

    /// Setup our rustyline instance, used for reading lines (yep)
    /// and managing history.
    fn setup_readline(&mut self) -> rustyline::Editor<()> {

        let config = rustyline::Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .build();

        let mut readline = rustyline::Editor::with_config(config).unwrap();

        if let Ok(home) = std::env::var("HOME") {
            let histfile = format!("{home}/{HISTORY_FILE}");
            readline.load_history(&histfile).ok(); // err() if not exists
            self.history_file = Some(histfile);
        }

        readline
    }

    /// Main entry point.
    fn main_loop(&mut self) {

        if let Err(e) = self.process_script_lines() {
            eprintln!("{e}");
            return;
        }

        let mut readline = self.setup_readline();

        loop {
            if let Err(e) = self.read_one_line(&mut readline) {
                eprintln!("Command failed: {e}");
            }
            self.spinner.hide();
        }
    }

    fn add_to_history(&self, readline: &mut rustyline::Editor<()>, line: &str) {

        readline.add_history_entry(line);

        if let Some(filename) = self.history_file.as_ref() {
            if let Err(e) = readline.append_history(filename) {
                eprintln!("Cannot append to history file: {e}");
            }
        }
    }

    fn process_script_lines(&mut self) -> Result<(), String> {

        // Avoid mucking with STDIN if we have no piped data to process.
        // Otherwise, it conflict with rustlyine.
        if atty::is(atty::Stream::Stdin) {
            return Ok(());
        }

        let mut buffer = String::new();
        let stdin = io::stdin();

        loop {
            buffer.clear();
            match stdin.read_line(&mut buffer) {
                Ok(count) => {

                    if count == 0 {
                        break; // EOF
                    }

                    let command = buffer.trim();

                    if command.len() == 0 {
                        // Empty line, but maybe still more data to process.
                        continue;
                    }

                    if let Err(e) = self.dispatch_command(&command) {
                        eprintln!("Error processing piped requests: {e}");
                        break;
                    }
                }

                Err(e) => return Err(format!("Error reading stdin: {e}"))
            }
        }

        // If we started on the receiving end of a pipe, exit after
        // all piped data has been processed, even if no usable
        // data was found.
        self.exit();

        Ok(())
    }

    /// Read a single line of user input and execute the command.
    ///
    /// If the command was successfully executed, return the command
    /// as a string so it may be added to our history.
    fn read_one_line(&mut self,
        readline: &mut rustyline::Editor<()>) -> Result<(), String> {

        let user_input = match readline.readline(PROMPT) {
            Ok(line) => line,
            Err(_) => return Ok(()),
        };

        self.spinner.show();

        let user_input = user_input.trim();

        if user_input.len() == 0 {
            return Ok(());
        }

        self.dispatch_command(&user_input)?;
        self.add_to_history(readline, &user_input);

        Ok(())
    }

    /// Route a command line to its handler.
    fn dispatch_command(&mut self, line: &str) -> Result<(), String> {
        let args: Vec<&str> = line.split(" ").collect();

        let command = args[0].to_lowercase();

        match command.as_str() {
            "stop" | "quit" | "exit" => {
                self.exit();
                Ok(())
            }
            "idl" => {
                self.idl_query(&args[..])
            }
            "db" => {
                self.db_command(&args[..])
            }
            "req" | "request" => {
                self.send_request(&args[..])
            }
            "router" => {
                self.send_router_command(&args[..])
            }
            "help" => {
                println!("{HELP_TEXT}");
                Ok(())
            }
            _ => Err(format!("Unknown command: {command}")),
        }
    }

    fn send_router_command(&mut self, args: &[&str]) -> Result<(), String> {
        self.check_command_length(args, 3)?;

        let mut node_name = args[1];
        let command = args[2];

        if node_name.eq("_") {
            let pc = self.config.primary_connection().unwrap();
            node_name = pc.node_name();
        }

        let router_class = match args.len() > 3 {
            true => Some(args[3]),
            false => None
        };

        // Assumes the caller wants to see the response for any
        // router request.
        if let Some(resp) =
            self.client.send_router_command(node_name, command, router_class, true)? {
            self.print_json_record(&resp);
        }

        Ok(())
    }

    fn send_request(&mut self, args: &[&str]) -> Result<(), String> {
        self.check_command_length(args, 3)?;

        let mut params: Vec<json::JsonValue> = Vec::new();

        let mut idx = 3;
        while idx < args.len() {
            let p = match json::parse(args[idx]) {
                Ok(p) => p,
                Err(e) => return Err(
                    format!("Cannot parse parameter: {} {}", args[idx], e)),
            };
            params.push(p);
            idx += 1;
        }

        let mut ses = self.client.session(args[1]);
        let mut req = ses.request(args[2], &params)?;
        while let Some(resp) = req.recv(DEFAULT_REQUEST_TIMEOUT)? {
            self.print_json_record(&resp);
        }

        Ok(())
    }

    fn db_command(&mut self, args: &[&str]) -> Result<(), String> {
        self.check_command_length(args, 3)?;

        match args[1].to_lowercase().as_str() {
            "sleep" => self.db_sleep(args[2]),
            _ => Err(format!("Unknown 'db' command: {args:?}"))
        }
    }

    fn db_sleep(&mut self, secs: &str) -> Result<(), String> {

        let secs: f64 = match secs.parse::<f64>() {
            Ok(s) => s,
            Err(_) => return Err(format!("Invalid sleep duration: {secs}")),
        };

        let db = match &mut self.db {
            Some(d) => d,
            None => return Err(format!("'db' command requires database connection"))
        };

        let query = "SELECT PG_SLEEP($1)";

        let query_res = db.borrow_mut().client().query(&query[..], &[&secs]);

        if let Err(e) = query_res {
            return Err(format!("DB query failed: {e}"));
        }

        Ok(())
    }

    /// Returns Err if the str slice does not contain enough entries.
    fn check_command_length(&self, args: &[&str], len: usize) -> Result<(), String> {
        if args.len() < len {
            Err(format!("Command is incomplete: {args:?}"))
        } else {
            Ok(())
        }
    }

    fn exit(&mut self) {
        std::process::exit(0x0);
    }

    /// Launch an IDL query.
    fn idl_query(&mut self, parts: &[&str]) -> Result<(), String> {
        self.check_command_length(&parts[..], 4)?;

        match parts[1] {
            "get" => self.idl_get(&parts[2..]),
            _ => return Err(format!("Could not parse idl query command: {parts:?}")),
        }
    }

    /// Retrieve a single IDL object by its primary key value
    fn idl_get(&mut self, parts: &[&str]) -> Result<(), String> {
        let classname = parts[0];
        let pkey = parts[1];

        let translator = match self.db_translator.as_mut() {
            Some(t) => t,
            None => return Err(format!("Database connection required")),
        };

        let obj = match translator.idl_class_by_pkey(classname, pkey)? {
            Some(o) => o,
            None => return Ok(())
        };

        self.print_json_record(&obj);
        Ok(())
    }

    fn print_json_record(&self, obj: &json::JsonValue) {
        println!("{}", obj.pretty(self.json_print_depth));
        println!("{SEPARATOR}");
    }

    fn print_idl_object(&self, obj: &json::JsonValue) -> Result<(), String> {

        let classname = match obj[idl::CLASSNAME_KEY].as_str() {
            Some(c) => c,
            None => return Err(format!(
                "Not a valid IDL object value: {}", obj.dump())),
        };

        let idl_class = match self.idl.classes().get(classname) {
            Some(c) => c,
            None => return Err(format!(
                "Object has an invalid class name {classname}")),
        };

        let fields = idl_class.real_fields_sorted();

        // Get the max field name length for improved formatting.
        let mut maxlen = 0;
        for field in fields.iter() {
            if field.name().len() > maxlen {
                maxlen = field.name().len();
            }
        };
        maxlen += 3;

        for field in idl_class.real_fields_sorted() {
            let name = field.name();
            let value = &obj[name];
            println!("{name:.<width$} {value}", width = maxlen);
        }

        println!("{SEPARATOR}");

        Ok(())
    }
}

