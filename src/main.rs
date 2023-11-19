use std::env;

fn decode_bencode_value(encoded_value: &str) -> serde_json::Value {
    if let Some((len, rest)) = encoded_value.split_once(':') {
        if let Ok(len) = len.parse::<usize>() {
            return serde_json::Value::String(rest[..len].to_string());
        }
    }
    panic!("Unhandled encoded value: {}", encoded_value);
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
