use json;
use opensrf::client::ClientHandle;
use super::event;
use super::error::Error;

const LOGIN_TIMEOUT: i32 = 30;

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

    pub fn login(client: &mut ClientHandle, args: &AuthLoginArgs) -> Result<AuthSession, String> {

        let mut params = vec![
            json::object! {
                username: args.username(),
                password: args.password(),
                type: args.login_type()
            }
        ];

        if let Some(w) = args.workstation() {
            params[0]["workstation"] = json::from(w);
        }

        let mut ses = client.session("open-ils.auth");
        let mut req = ses.request("open-ils.auth.login",  params)?;

        let json_val = match req.recv(LOGIN_TIMEOUT)? {
            Some(v) => v,
            None => { return Err("Login Timed Out".to_string()); }
        };

        let evt = match event::EgEvent::parse(&json_val) {
            Some(e) => e,
            None => {
                return Err(format!("Unexpected response: {:?}", json_val));
            }
        };

        if !evt.success() {
            return Err(format!("Non-success event returned"));
        }

        if !evt.payload().is_object() {
            return Err(format!("Unexpected response: {}", evt));
        }

        let token = match evt.payload()["authtoken"].as_str() {
            Some(t) => String::from(t),
            None => {
                return Err(format!("Unexpected response: {}", evt));
            }
        };

        let authtime = match evt.payload()["authtime"].as_usize() {
            Some(t) => t,
            None => {
                return Err(format!("Unexpected response: {}", evt));
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

