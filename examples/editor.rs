use eg::idl;
use evergreen as eg;
use opensrf::Client;
use opensrf::ClientConfig;

fn main() -> Result<(), String> {
    let conf = ClientConfig::from_file("conf/opensrf_client.yml")?;

    let idl = idl::Parser::parse_file("/openils/conf/fm_IDL.xml")?;

    let mut client = Client::new(conf)?;

    client.set_serializer(idl::Parser::as_serializer(&idl));

    let args = eg::auth::AuthLoginArgs::new("admin", "demo123", "temp", None);
    let auth_ses = eg::auth::AuthSession::login(&mut client, &args)?;

    println!("\nLogged in and got authtoken: {}", auth_ses.token());

    let mut editor = eg::Editor::with_auth(&client, &idl, auth_ses.token());
    if editor.checkauth()? {
        println!("Auth Check OK: {}", editor.requestor().unwrap()["usrname"]);
    }

    if let Some(org) = editor.retrieve("aou", json::from(4))? {
        println!("Fetched org unit: {}", org["shortname"]);
    }

    Ok(())
}
