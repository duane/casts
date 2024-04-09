use casts::graphql::create_schema;

const USAGE: &str = "USAGE: introspect SCHEMA|JSON";

fn main() {
    match std::env::args().nth(1).expect(USAGE).as_str() {
        "schema" => println!("{}", create_schema().as_schema_language()),
        "parser" => println!("{}", create_schema().as_parser_document()),
        _ => unimplemented!("unimplemented {}", USAGE),
    }
}
