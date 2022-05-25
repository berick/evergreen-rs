use evergreen::idl;
use opensrf::client::Client;
use opensrf::conf::ClientConfig;

fn main() {
    let mut conf = ClientConfig::new();
    conf.load_file("conf/opensrf_client.yml")
        .expect("Error loading config");
    //conf.load_xml_file("/openils/conf/opensrf_core.xml", "service");

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf.bus_config()).unwrap();
    client.serializer = Some(&parser);

    let ses = client.session("open-ils.cstore");

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

    /*
    while !client.complete(&req) {
        match client.recv(&req, 10).unwrap() {
            Some(user) => println!("{} {} home_ou={}",
                user["id"], user["usrname"], user["home_ou"]["name"]),
            None => break // timeout or complete
        }
    }
    */

    client.cleanup(&ses);
}
