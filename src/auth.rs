use json;
use opensrf::client::Client;
use super::event;

pub struct AuthLoginArgs {
    pub username: String,
    pub password: String,
    pub login_type: String, // "type" in the API
    pub workstation: Option<String>,
}

impl AuthLoginArgs {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }

    pub fn login_type(&self) -> &str {
        &self.login_type
    }

    pub fn workstation(&self) -> Option<&str> {
        self.workstation.as_deref()
    }
}

pub struct AuthSession {
    token: String,
    authtime: usize,
    workstation: Option<String>,
}

impl AuthSession {

    // TODO error: types, etc.
    pub fn login(client: &mut Client, args: &AuthLoginArgs) -> Result<AuthSession, ()> {

        let mut params = vec![
            json::object! {
                username: json::from(args.username()),
                password: json::from(args.password()),
                type: json::from(args.login_type())
            }
        ];

        if let Some(w) = args.workstation() {
            params[0]["workstation"] = json::from(w);
        }

        let ses = client.session("open-ils.auth");

        let req = client
            .request(&ses, "open-ils.auth.login",  params)
            .expect("Cannot create OpenSRF request"); // TODO error:
            // TODO ses cleanup has to happen

        let evt = match client.recv(&req, 60) { // TODO timeout
            Ok(op) => {
                match event::EgEvent::parse(op) {
                    Some(evt) => evt,
                    None => {
                        client.cleanup(&ses);
                        return Err(()); // TODO
                    }
                }
            },
            Err(e) => {
                client.cleanup(&ses);
                return Err(()); // TODO
            }
        };

        // TODO avoid early exit so we can always cleanup
        client.cleanup(&ses);

        if !evt.success() {
            return Err(()); // TODO
        }

        if !evt.payload().is_object() {
            return Err(()); // TODO
        }

        let token = match evt.payload()["authtoken"].as_str() {
            Some(t) => String::from(t),
            None => {
                return Err(()); // TODO
            }
        };

        let authtime = match evt.payload()["authtime"].as_usize() {
            Some(t) => t,
            None => {
                return Err(()); // TODO
            }
        };

        let mut auth_ses = AuthSession {
            token: token,
            authtime: authtime,
            workstation: None,
        };

        if let Some(w) = &args.workstation {
            auth_ses.workstation = Some(String::from(w));
        }

        Ok(auth_ses)
    }

    pub fn token(&self) -> &str {
        &self.token
    }

    pub fn authtime(&self) -> usize {
        self.authtime
    }

    pub fn workstation(&self) -> Option<&str> {
        self.workstation.as_deref()
    }
}

