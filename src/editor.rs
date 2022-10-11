use opensrf as osrf;
use super::event::EgEvent;
use json::JsonValue;
//use log::{debug, error, trace, warn};
use std::cell::RefCell;
use std::rc::Rc;
//use std::fmt;

const DEFAULT_EDITOR_PERSONALITY: &str = "open-ils.cstore";

pub struct Editor {
    //client: Rc<RefCell<osrf::Client>>,
    client: osrf::ClientHandle,
    personality: String,
    authtoken: Option<String>,
    authtime: Option<usize>,
    requestor: Option<JsonValue>,
    last_event: Option<EgEvent>,
}

impl Editor {

    pub fn checkauth(&mut self) -> Result<bool, String> {

        if self.authtoken.is_none() {
            return Ok(false);
        }

        let token = self.authtoken().unwrap();

        let service = "open-ils.auth";
        let method = "open-ils.auth.session.retrieve";
        let params = vec![json::from(token), json::from(true)];

        let resp_op = self.client.sendrecv(service, method, params)?.next();

        let resp = match resp_op {
            Some(r) => r,
            None => {
                return Err(format!("No response from auth server in checkauth()"));
            }
        };

        let evt = match EgEvent::parse(&resp) {
            Some(e) => e,
            None => {
                return Err(format!(
                    "Auth session check returned unexpected response: {resp:?}"));
            }
        };

        if evt.textcode().ne("SUCCESS") {
            // TODO set last_event
            return Ok(false);
        }

        let payload = evt.payload();

        // Auth session is valid.  Let's update some local data
        self.authtime = osrf::util::json_usize(&payload["authtime"]);
        self.requestor = Some(payload["userobj"].to_owned());

        Ok(true)
    }

    pub fn authtoken(&self) -> Option<&str> {
        self.authtoken.as_deref()
    }
}

