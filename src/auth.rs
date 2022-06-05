use json;
use opensrf::client::Client;
use super::event;
use super::error::Error;

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

    pub fn login(client: &mut Client, args: &AuthLoginArgs) -> Result<AuthSession, Error> {

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

        let req = match client.request(&ses, "open-ils.auth.login",  params) {
            Ok(r) => r,
            Err(e) => {
                client.cleanup(&ses);
                return Err(Error::OpenSrfError(e));
            }
        };

        // TODO global default timeout? / redo to accep None
        let recv_op = match client.recv(&req, 60) {
            Ok(op) => op,
            Err(e) => {
                client.cleanup(&ses);
                return Err(Error::OpenSrfError(e));
            }
        };

        client.cleanup(&ses); // All done w/ this session.

        let json_val = match recv_op {
            Some(v) => v,
            None => {
                return Err(Error::LoginFailedError(
                    Some(format!("No response resturned"))));
            }
        };

        let evt = match event::EgEvent::parse(&json_val) {
            Some(e) => e,
            None => {
                return Err(Error::LoginFailedError(
                    Some(format!("Unexpected response: {:?}", json_val))));
            }
        };

        if !evt.success() {
            return Err(Error::LoginFailedError(None));
        }

        if !evt.payload().is_object() {
            return Err(Error::LoginFailedError(
                Some(format!("Unexpected response: {}", evt))));
        }

        let token = match evt.payload()["authtoken"].as_str() {
            Some(t) => String::from(t),
            None => {
                return Err(Error::LoginFailedError(
                    Some(format!("Unexpected response: {}", evt))));
            }
        };

        let authtime = match evt.payload()["authtime"].as_usize() {
            Some(t) => t,
            None => {
                return Err(Error::LoginFailedError(
                    Some(format!("Unexpected response: {}", evt))));
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

