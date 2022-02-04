use opensrf::conf::ClientConfig;
use opensrf::client::Client;
use opensrf::client::ClientSession;
use opensrf::client::DataSerializer;
use evergreen::idl;

const USER_JSON: &str = r#"{"__c":"au","__p":[[1,2,3,"yes"],null,null,null,null,null,null,null,null,null,null,null,"t","f",null,1,0,0,"2022-01-08T13:40:06-0500","0.00",null,"1979-01-22",null,null,"2025-01-08T13:40:06-0500","SystemAccount","Administrator",{"__c":"aou","__p":[null,1,1,1,1,1,"ExampleConsortium",1,null,"CONS",null,null,"t",1]},1,1,null,"identification",null,"none",null,"t",1,null,"a16e3d5fd48c3709855656e9000f4951",null,null,1,null,1,null,"t",1,"admin",null,"f","2022-01-08T13:40:09-0500",null,null,null,null,null,null,null,"'account':4'administr':1,2'system':3'systemaccount':5",null,"f"]}"#;


fn main() {

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");

    /*
    let user_encoded = &json::parse(USER_JSON).unwrap();
    let mut user_hash = parser.unpack(user_encoded);

    println!("BUIT: {}", user_hash);

    user_hash["usrname"] = json::from("HAMBONE");

    let user_encoded = parser.pack(&user_hash);

    println!("AND: {}", user_encoded);
    */

    let mut conf = ClientConfig::new();
    conf.load_file("conf/opensrf_client.yml");

    let mut client = Client::new(conf.bus_config()).unwrap();
    client.serializer = Some(&parser);

    let ses = client.session("open-ils.cstore");

    let params = vec![json::from(1)];
    let req = client.request(&ses,
        "open-ils.cstore.direct.actor.user.retrieve", params).unwrap();

    while !client.complete(&req) {
        match client.recv(&req, 10).unwrap() {
            Some(value) => {
                println!("REQ2 GOT RESPONSE: {}", value.dump());
            },
            None => {
                println!("req returned None");
                break;
            }
        }
    }

    client.cleanup(&ses);
}

