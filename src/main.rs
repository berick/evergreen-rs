use opensrf::conf::ClientConfig;
use opensrf::client::Client;
use opensrf::client::ClientSession;
use opensrf::client::DataSerializer;
use evergreen::idl;

fn main() {
    let mut conf = ClientConfig::new();
    conf.load_file("conf/opensrf_client.yml");

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf.bus_config()).unwrap();
    client.serializer = Some(&parser);

    let ses = client.session("open-ils.cstore");

    let params = vec![
        json::from(1),
        json::object!{
            flesh: 1,
            flesh_fields: json::object!{
                au: vec!["home_ou"]
            }
        }
    ];

    let req = client.request(&ses,
        "open-ils.cstore.direct.actor.user.retrieve", params).unwrap();

    while !client.complete(&req) {
        match client.recv(&req, 10).unwrap() {
            Some(user) => println!("{} {} home_ou={}",
                user["id"], user["usrname"], user["home_ou"]["name"]),
            None => break
        }
    }

    client.cleanup(&ses);
}

