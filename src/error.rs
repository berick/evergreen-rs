use opensrf as osrf;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
	OpenSrfError(osrf::error::Error),
    LoginFailedError(Option<String>),
}

use self::Error::*;

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            OpenSrfError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenSrfError(ref err) => err.fmt(f),
            LoginFailedError(op) => {
                match op {
                    Some(s) => write!(f, "Login Failed: {}", s),
                    None => write!(f, "Login Failed")
                }
            },
		}
    }
}


