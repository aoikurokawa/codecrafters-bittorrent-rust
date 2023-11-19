use std::env;

fn decode_bencode_value(encoded_value: &str) -> (serde_json::Value, &str) {
    match encoded_value.chars().next() {
        Some('i') => {
            if let Some((n, rest)) =
                encoded_value
                    .split_at(1)
                    .1
                    .split_once('e')
                    .and_then(|(digits, rest)| {
                        let n = digits.parse::<i64>().ok()?;
                        Some((n, rest))
                    })
            {
                return (n.into(), rest);
            }
        }
        Some('l') => {
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && !rest.starts_with('e') {
                let (v, remainder) = decode_bencode_value(rest);
                values.push(v);
                rest = remainder;
            }
            return (values.into(), &rest[1..]);
        }
        Some('d') => {
            let mut dict = serde_json::Map::new();
            let mut values = Vec::new();
            let mut rest = encoded_value.split_at(1).1;
            while !rest.is_empty() && !rest.starts_with('e') {
                let (k, remainder) = decode_bencode_value(rest);
                let k = match k {
                    serde_json::Value::String(k) => k,
                    k => {
                        panic!("key must be strings, not {k:?}");
                    }
                };
                let (v, remainder) = decode_bencode_value(remainder);
                dict.insert(k, v.clone());
                values.push(v);
                rest = remainder;
            }
            return (dict.into(), &rest[1..]);
        }
        Some('0'..='9') => {
            if let Some((len, rest)) = encoded_value.split_once(':').and_then(|(len, rest)| {
                let len = len.parse::<usize>().ok()?;
                Some((len, rest))
            }) {
                return (rest[..len].to_string().into(), &rest[len..]);
            }
        }
        _ => {}
    }
    panic!("Unhandled encoded value: {}", encoded_value);
}

struct Torrent {
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        eprintln!("Logs from your program will appear here!");

        let encoded_value = &args[2];
        let decoded_value = decode_bencode_value(&encoded_value);
        println!("{}", decoded_value.0);
    } else {
        eprintln!("unknown command: {}", args[1]);
    }
}
