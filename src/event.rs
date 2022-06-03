use json;
use std::fmt;

pub struct EgEvent {
    code: isize,
    textcode: String,
    payload: json::JsonValue, // json::JsonValue::Null if empty
    desc: Option<String>,
    debug: Option<String>,
    note: Option<String>,
    servertime: Option<String>,
    ilsperm: Option<String>,
    ilspermloc: isize,
    success: bool,
}

impl fmt::Display for EgEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = format!("Event: {}:{}", self.code, self.textcode);

        if let Some(ref d) = self.desc {
            s = s + " -> " + d;
        }

        if let Some(ref n) = self.note {
            s = s + "\n" + n;
        }

        write!(f, "{}", s)
    }
}

impl EgEvent {

    pub fn parse(thing: Option<json::JsonValue>) -> Option<EgEvent> {

        if thing.is_none() { return None; }

        let jv: json::JsonValue = thing.unwrap();

        if !jv.is_object() { return None; }

        // textcode is the only required field.
        let textcode = match jv["textcode"].as_str() {
            Some(c) => String::from(c),
            _ => { return None; }
        };

        let success = textcode.eq("SUCCESS");

        let mut evt = EgEvent {
            code: -1,
            textcode: textcode,
            payload: jv["payload"].clone(),
            desc: None,
            debug: None,
            note: None,
            servertime: None,
            ilsperm: None,
            ilspermloc: -1,
            success: success,
        };

        if let Some(code) = jv["ilsevent"].as_isize() {
            evt.code = code;
        };

        if let Some(permloc) = jv["ilspermloc"].as_isize() {
            evt.ilspermloc = permloc;
        }

        for field in vec!["desc", "debug", "note", "servertime", "ilsperm"] {
            if let Some(value) = jv[field].as_str() {

                let v = String::from(value);
                match field {
                    "desc" => evt.desc = Some(v),
                    "debug" => evt.debug = Some(v),
                    "note" => evt.note = Some(v),
                    "servertime" => evt.servertime = Some(v),
                    "ilsperm" => evt.ilsperm = Some(v),
                    _ => {},
                }
            }
        }

        Some(evt)
    }
}
