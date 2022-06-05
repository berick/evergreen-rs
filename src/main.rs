use evergreen as eg;
use opensrf as osrf;
use osrf::client::Client;
use osrf::conf::ClientConfig;

fn main() {
    let mut conf = ClientConfig::new();

    conf.load_file("conf/opensrf_client.yml")
        .expect("Error loading config");

    let parser = eg::idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf.bus_config()).expect("Cannot connect to OpenSRF Bus");

    client.serializer = Some(&parser);

    let ses = client.session("open-ils.cstore");

    let params = vec![
        json::object! {
            id: vec![
                json::from(1),
                json::from(2),
                json::from(3)
            ]
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
    client.connect(&ses).expect("Connect failed");

    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.search", params)
        .expect("Cannot create OpenSRF request");

    // The fast-and-loose way that can lead to a (possibly unwanted) panic
    // and/or a request timeout appearing as if the request is complete.
    while let Some(user) = client.recv(&req, 10).expect("recv() failed") {
        println!(
            "{} {} home_ou={}",
            user["id"], user["usrname"], user["home_ou"]["name"]
        );
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

    println!("Logged in and go authtoken: {}", auth_ses.token());

}
