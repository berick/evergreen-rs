use evergreen::idl;
use opensrf::client::Client;
use opensrf::conf::ClientConfig;

fn main() {
    let mut conf = ClientConfig::new();

    conf.load_file("conf/opensrf_client.yml").expect("Error loading config");

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf.bus_config())
        .expect("Cannot connect to OpenSRF Bus");

    client.serializer = Some(&parser);

    let ses = client.session("open-ils.cstore");

    /*
    let params = vec![
        json::from(1),
        json::object! {
            flesh: 1,
            flesh_fields: json::object!{
                au: vec!["home_ou"]
            }
        },
    ];

    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.retrieve", params)
        .unwrap();

    if let Some(user) = client.recv_one(&req, 10).unwrap() {
        println!(
            "Fetched user id={} usrname={} homeorg={}",
            user["id"], user["usrname"], user["home_ou"]["shortname"]
        );
    }
    */

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

    let params2 = params.clone();

    client.connect(&ses);

    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.search", params)
        .expect("Cannot create OpenSRF request");

    while !client.complete(&req) {
        match client.recv(&req, 10).unwrap() {
            Some(user) => println!("{} {} home_ou={}",
                user["id"], user["usrname"], user["home_ou"]["name"]),
            None => break // timeout or complete
        }
    }

    let req = client
        .request(&ses, "open-ils.cstore.direct.actor.user.search", params2)
        .expect("Cannot create OpenSRF request");

    while !client.complete(&req) {
        match client.recv(&req, 10).unwrap() {
            Some(user) => println!("{} {} home_ou={}",
                user["id"], user["usrname"], user["home_ou"]["name"]),
            None => break // timeout or complete
        }
    }

    client.disconnect(&ses);

    // Remove session data from the local cache so it doesn't
    // slowly build over time.
    client.cleanup(&ses);
}

