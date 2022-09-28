use evergreen as eg;
use opensrf as osrf;
use osrf::client::Client;
use osrf::conf::ClientConfig;
use std::rc::Rc;
use std::cell::RefCell;

fn main() {
    let mut conf = ClientConfig::new();

    conf.load_file("conf/opensrf_client.yml").expect("Error loading config");

    let idl = eg::idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    // Wrap IDL in an Rc() so we can share refs to it for various purposes
    // It's too big to clone and in practice will never be freed anyway.
    //let serializer: Rc<RefCell<dyn osrf::client::DataSerializer>> = idlref.clone();

    let mut client = Client::new(conf).expect("Cannot connect to OpenSRF Bus");

    client.set_serializer(idl.as_serializer());

    println!("parser class count = {}", idl.parser().classes().len());

    let mut ses = client.session("open-ils.cstore");

    let params = vec![
        json::object! {
            id: vec![1, 2, 3]
        },
        json::object! {
            flesh: 1,
            flesh_fields: json::object!{
                au: vec!["home_ou"]
            }
        },
    ];

    // request() consumes its params; clone these so we can use them twice
    let params2 = params.clone();

    // optional -- testing
    ses.connect().unwrap();

    let mut req =
        ses.request("open-ils.cstore.direct.actor.user.search", params).unwrap();

    while let Some(user) = req.recv(10).unwrap() {
        println!(
            "{} {} home_ou={}",
            user["id"], user["usrname"], user["home_ou"]["name"]
        );
    }

    ses.disconnect();

    /*


    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.search", params)
        .expect("Cannot create OpenSRF request");

    // The fast-and-loose way that can lead to a (possibly unwanted) panic
    // and/or a request timeout appearing as if the request is complete.
    while let Some(user) = client.recv(&req, 10).expect("recv() failed") {
    }

    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.search", params2)
        .expect("Cannot create OpenSRF request");

    // The more verbose, all hands on deck approach.
    while !client.complete(&req) {
        match client.recv(&req, 10) {
            Ok(recv_op) => {
                match recv_op {
                    Some(user) => {
                        println!(
                            "{} {} home_ou={}",
                            user["id"], user["usrname"], user["home_ou"]["name"]
                        );
                    }
                    None => {
                        // Could be a timeout OR the request just completed.
                        // Let the base while {} determine.
                    }
                }
            }
            Err(e) => {
                eprintln!("recv() returned an error: {}", e);
                break;
            }
        }
    }

    // only needed if connect() is called above.
    client.disconnect(&ses).expect("Disconnect failed");

    // Remove session data from the local cache so it doesn't
    // slowly build over time.
    client.cleanup(&ses); // Required

    let args = eg::auth::AuthLoginArgs {
        username: String::from("admin"),
        password: String::from("demo123"),
        login_type: String::from("temp"),
        workstation: None,
    };

    let auth_ses = eg::auth::AuthSession::login(&mut client, &args).expect("Login Error");

    println!("Logged in and got authtoken: {}", auth_ses.token());
    */

}
