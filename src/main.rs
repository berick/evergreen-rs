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

    // only needed if connect() is called.
    client.disconnect(&ses).expect("Disconnect failed");

    // Remove session data from the local cache so it doesn't
    // slowly build over time.
    client.cleanup(&ses); // Required when done w/ a session
}

