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

    let mut client = Client::new(conf).expect("Cannot connect to OpenSRF Bus");

    client.set_serializer(idl.as_serializer());

    println!("parser class count = {}", idl.parser().classes().len());

    let mut ses = client.session("open-ils.storage");

    let mut req = ses.request("opensrf.system.echo", vec!["howdy", "world"]).unwrap();

    while let Some(txt) = req.recv(10).unwrap() {
        println!("Echo returned: {txt:?}");
    }

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
    //ses.connect().unwrap();

    let mut req =
        ses.request("open-ils.storage.direct.actor.user.search", params).unwrap();

    while let Some(user) = req.recv(10).unwrap() {
        println!(
            "{} {} home_ou={}",
            user["id"], user["usrname"], user["home_ou"]
            //user["id"], user["usrname"], user["home_ou"]["name"]
        );
    }

    //ses.disconnect();

    /*

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
