use std::env;

fn decode_bencode_value(value: &str) -> &str {
    ""
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        println!("Logs from your program will appear here!");

        let encoded_value = &args[2];
        let decoded_value = decode_bencode_value(&encoded_value);
        println!("{}", decoded_value);
    } else {
        println!("unknown command: {}", args[1]);
    }
}
