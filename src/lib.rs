use opensrf::conf::ClientConfig;
use opensrf::client::Client;

pub mod idl;

// TODO getops
/*
pub fn init() -> Client<'static> {

    let mut conf = ClientConfig::new();
    conf.load_file("conf/opensrf_client.yml");

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    let mut client = Client::new(conf.bus_config()).unwrap();
    client.serializer = Some(&parser);

    client
}
*/
