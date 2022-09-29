use evergreen as eg;
use opensrf as osrf;
use osrf::client::Client;
use osrf::conf::ClientConfig;

fn main() -> Result<(), String> {
    let mut conf = ClientConfig::new();

    conf.load_file("conf/opensrf_client.yml")?;

    let idl = eg::idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf)?;

    client.set_serializer(idl.as_serializer());

    println!("parser class count = {}", idl.parser().classes().len());

    let mut ses = client.session("open-ils.cstore");

    let mut req = ses.request("opensrf.system.echo", vec!["howdy", "world"])?;

    while let Some(txt) = req.recv(10)? {
        println!("Echo returned: {txt:?}");
    }

    let method = "open-ils.cstore.direct.actor.user.search";

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

    // optional -- testing
    //ses.connect()?;

    for idx in 0..9 {
        // Iterator example
        for user in ses.sendrecv(method, params.clone())? {
            println!(
                "{} {} home_ou={}",
                user["id"], user["usrname"], user["home_ou"]["name"]
            );
        }
    }

    // Manual request management example
    let mut req = ses.request(method, params)?;

    while let Some(user) = req.recv(10)? {
        println!(
            "{} {} home_ou={}",
            user["id"], user["usrname"], user["home_ou"]["name"]
        );
    }

    //ses.disconnect()?; // Only needed if ses.connect() is called.

    let args = eg::auth::AuthLoginArgs {
        username: String::from("admin"),
        password: String::from("demo123"),
        login_type: String::from("temp"),
        workstation: None,
    };

    let auth_ses = eg::auth::AuthSession::login(&mut client, &args)?;

    println!("\nLogged in and got authtoken: {}", auth_ses.token());

    Ok(())
}
