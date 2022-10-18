use eg::idl;
use evergreen as eg;
use opensrf::Client;
use opensrf::Config;
use opensrf::Logger;

fn main() -> Result<(), String> {
    let mut conf = Config::from_file("conf/opensrf_client.yml")?;
    let con = conf.set_primary_connection("service", "private.localhost")?;

    let ctype = con.connection_type();
    Logger::new(ctype.log_level(), ctype.log_facility()).init().unwrap();

    let idl = idl::Parser::parse_file("/openils/conf/fm_IDL.xml")?;

    let mut client = Client::new(conf)?;

    client.set_serializer(idl::Parser::as_serializer(&idl));

    println!("parser class count = {}", idl.classes().len());

    let mut ses = client.session("open-ils.cstore");

    ses.connect()?;

    let mut req = ses.request("opensrf.system.echo", vec!["howdy", "world"])?;

    while let Some(txt) = req.recv(10)? {
        println!("Echo returned: {txt:?}");
    }

    ses.disconnect()?;

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

    for _ in 0..9 {
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

    let args = eg::auth::AuthLoginArgs::new("admin", "demo123", "temp", None);

    let auth_ses = eg::auth::AuthSession::login(&mut client, &args)?;

    println!("\nLogged in and got authtoken: {}", auth_ses.token());

    Ok(())
}
