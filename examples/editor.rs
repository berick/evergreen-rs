use eg::idl;
use evergreen as eg;
use opensrf::Client;
use opensrf::ClientConfig;

fn main() -> Result<(), String> {
    let conf = ClientConfig::from_file("conf/opensrf_client.yml")?;

    println!("Parsing IDL");
    let idl = idl::Parser::parse_file("/openils/conf/fm_IDL.xml")?;
    println!("Done parsing IDL");

    let mut client = Client::new(conf)?;

    client.set_serializer(idl::Parser::as_serializer(&idl));

    println!("Logging in...");

    let args = eg::auth::AuthLoginArgs::new("admin", "demo123", "temp", None);
    let auth_ses = eg::auth::AuthSession::login(&mut client, &args)?;
    let token = auth_ses.token();

    println!("Logged in and got authtoken: {}", token);

    let mut editor = eg::Editor::with_auth(&client, &idl, token);

    if editor.checkauth()? {
        println!("Auth Check OK: {}", editor.requestor().unwrap()["usrname"]);
    }

    if let Some(org) = editor.retrieve("aou", json::from(4))? {
        println!("Fetched org unit: {}", org["shortname"]);
    }

    let query = json::object!{"id": json::object!{"<": 10}};
    for perm in editor.search("ppl", query)? {
        println!("Search found permission: {}", perm["code"]);
    }

    Ok(())
}
