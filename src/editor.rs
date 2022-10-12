use opensrf as osrf;
use super::event::EgEvent;
use json::JsonValue;
//use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Personality {
    Cstore,
    Pcrud,
    ReporterStore,
}

impl From<&str> for Personality {
    fn from(s: &str) -> Self {
        match s {
            "open-ils.pcrud" => Self::Pcrud,
            "open-ils.reporter-store" => Self::ReporterStore,
            _ => Self::Cstore
        }
    }
}

impl From<&Personality> for &str {
    fn from(p: &Personality) -> &'static str {
        match *p {
            Personality::Cstore => "open-ils.cstore",
            Personality::Pcrud => "open-ils.pcrud",
            Personality::ReporterStore => "open-ils.reporter-store",
        }
    }
}

pub struct Editor {
    client: osrf::ClientHandle,
    personality: Personality,
    authtoken: Option<String>,
    authtime: Option<usize>,
    requestor: Option<JsonValue>,

    /// Most recent non-success event
    last_event: Option<EgEvent>,
}

impl Editor {

    pub fn new(client: &osrf::ClientHandle) -> Self {
        Editor {
            client: client.clone(),
            personality: "".into(),
            authtoken: None,
            authtime: None,
            requestor: None,
            last_event: None,
        }
    }

    pub fn new_with_auth(client: &osrf::ClientHandle, authtoken: &str) -> Self {
        let mut editor = Editor::new(client);
        editor.authtoken = Some(authtoken.to_string());
        editor
    }

    pub fn checkauth(&mut self) -> Result<bool, String> {

        let token = match self.authtoken() {
            Some(t) => t,
            None => { return Ok(false); }
        };

        let service = "open-ils.auth";
        let method = "open-ils.auth.session.retrieve";
        let params = vec![json::from(token), json::from(true)];

        let resp_op = self.client.sendrecv(service, method, params)?.next();

        if let Some(ref user) = resp_op {

            if let Some(evt) = EgEvent::parse(&user) {
                log::debug!("Editor checkauth call returned non-success event: {}", evt);
                self.set_last_event(evt);
                return Ok(false);
            }

            if user.has_key("usrname") {
                self.requestor = Some(user.to_owned());
                return Ok(true);
            }
        }

        log::debug!("Editor checkauth call returned unexpected data: {resp_op:?}");

        self.set_last_event(EgEvent::new("NO_SESSION"));
        Ok(false)

        /*


        let resp = match resp_op {
            Some(r) => r,
            None => {
                return Err(format!("No response from auth server in checkauth()"));
            }
        };

        log::trace!("Editor checkauth got: {resp}");

        let evt = match EgEvent::parse(&resp) {
            Some(e) => e,
            None => EgEvent::new("NO_SESSION"),
        };

        log::debug!("Editor checkauth() got {}", evt);

        if evt.textcode().ne("SUCCESS") {
            self.last_event = Some(evt);
            return Ok(false);
        }

        let payload = evt.payload();

        // Auth session is valid.  Update some local data.
        self.authtime = osrf::util::json_usize(&payload["authtime"]);
        self.requestor = Some(payload["userobj"].to_owned());

        Ok(true)
        */
    }

    pub fn authtoken(&self) -> Option<&str> {
        self.authtoken.as_deref()
    }

    pub fn authtime(&self) -> Option<usize> {
        self.authtime
    }

    pub fn requestor(&self) -> Option<&JsonValue> {
        match &self.requestor {
            Some(r) => Some(r),
            None => None
        }
    }

    fn set_last_event(&mut self, evt: EgEvent) {
        self.last_event = Some(evt);
    }
}

