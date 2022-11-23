use super::idl;
use getopts;
use opensrf as osrf;
use std::env;
use std::sync::Arc;

const DEFAULT_IDL_PATH: &str = "/openils/conf/fm_IDL.xml";

pub struct Context {
    client: osrf::client::Client,
    config: Arc<osrf::conf::Config>,
    idl: Arc<idl::Parser>,
    params: getopts::Matches,
}

impl Context {
    pub fn client(&self) -> &osrf::client::Client {
        &self.client
    }
    pub fn config(&self) -> &Arc<osrf::conf::Config> {
        &self.config
    }
    pub fn idl(&self) -> &Arc<idl::Parser> {
        &self.idl
    }
    pub fn params(&self) -> &getopts::Matches {
        &self.params
    }
}

pub struct InitOptions {
    osrf_ops: osrf::init::InitOptions,
}

impl InitOptions {
    pub fn new() -> Self {
        InitOptions {
            osrf_ops: osrf::init::InitOptions::new(),
        }
    }
}

/// Read common command line parameters, parse the core config, apply
/// the primary connection type, and setup logging.
pub fn init() -> Result<Context, String> {
    init_with_options(&mut getopts::Options::new())
}

pub fn init_with_options(opts: &mut getopts::Options) -> Result<Context, String> {
    init_with_more_options(opts, &InitOptions::new())
}

/// Same as init(), but allows the caller to pass in a prepopulated set
/// of getopts::Options, which are then augmented with the standard
/// OpenSRF command line options.
pub fn init_with_more_options(
    opts: &mut getopts::Options,
    options: &InitOptions,
) -> Result<Context, String> {
    // Get the IDL from opensrf.settings, but allow the caller to
    // manually override for testing, etc. purposes.
    opts.optopt("", "idl-file", "Path to IDL file", "IDL_PATH");

    let (config, _) = osrf::init::init_with_more_options(opts, &options.osrf_ops)?;
    let config = config.into_shared();

    let args: Vec<String> = env::args().collect();
    let params = opts.parse(&args[1..]).unwrap();

    let client = osrf::Client::connect(config.clone())
        .or_else(|e| Err(format!("Cannot connect to OpenSRF: {e}")))?;

    // TODO pull the IDL path from opensrf.settings, while allowing
    // for override for testing purposes.
    let idl_file = params
        .opt_get_default("idl-file", DEFAULT_IDL_PATH.to_string())
        .unwrap();

    let idl = idl::Parser::parse_file(&idl_file)
        .or_else(|e| Err(format!("Cannot parse IDL file: {e}")))?;

    client.set_serializer(idl::Parser::as_serializer(&idl));

    Ok(Context {
        client,
        params,
        config,
        idl,
    })
}
