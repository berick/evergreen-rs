use evergreen::idl;

const USER_JSON: &str = r#"{"__c":"au","__p":[[1,2,3,"yes"],null,null,null,null,null,null,null,null,null,null,null,"t","f",null,1,0,0,"2022-01-08T13:40:06-0500","0.00",null,"1979-01-22",null,null,"2025-01-08T13:40:06-0500","SystemAccount","Administrator",{"__c":"aou","__p":[null,1,1,1,1,1,"ExampleConsortium",1,null,"CONS",null,null,"t",1]},1,1,null,"identification",null,"none",null,"t",1,null,"a16e3d5fd48c3709855656e9000f4951",null,null,1,null,1,null,"t",1,"admin",null,"f","2022-01-08T13:40:09-0500",null,null,null,null,null,null,null,"'account':4'administr':1,2'system':3'systemaccount':5",null,"f"]}"#;


fn main() {

    let parser = idl::Parser::parse_file("/openils/conf/fm_IDL.xml");
    let user_encoded = &json::parse(USER_JSON).unwrap();
    let user_hash = parser.unpack(user_encoded);

    println!("BUIT: {}", user_hash);

    let user_encoded = parser.pack(&user_hash);

    println!("AND: {}", user_encoded);

}

