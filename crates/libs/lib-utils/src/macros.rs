#[macro_export]
macro_rules! str_concat {
    ($($x:expr),*) => {
        {
            let mut string_final = String::with_capacity(512);
            $(
                string_final.push_str($x);
            )*
            string_final
        }
    };
}

#[macro_export]
/// Base64 decode
/// # Param: 
///  + `data`
///  + `padding`: `base64::engine::general_purpose::{}`, `STANDARD`(default) / `STANDARD_NO_PAD` / `URL_SAFE` / `URL_SAFE_NO_PAD`
macro_rules! b64_encode {
    ($data:expr) => {
        b64_encode!($data, base64::engine::general_purpose::STANDARD)
    };
    ($data:expr, $padding:path) => {
        base64::Engine::encode(&$padding, $data)
    };
}

#[macro_export]
/// Base64 decode
/// # Param: 
///  + `data`
///  + `padding`: `base64::engine::general_purpose::{}`, `STANDARD`(default) / `STANDARD_NO_PAD` / `URL_SAFE` / `URL_SAFE_NO_PAD`
macro_rules! b64_decode {
    ($data:expr) => {
        b64_decode!($data, base64::engine::general_purpose::STANDARD)
    };
    ($data:expr, $padding:path) => {
        base64::Engine::decode(&$padding, $data)
    };
}

#[macro_export]
macro_rules! calc_md5 {
    ($input_str: expr) => {{
        // let mut md5_instance = crypto::md5::Md5::new();
        // crypto::digest::Digest::input_str(&mut md5_instance, &($input_str));
        // crypto::digest::Digest::result_str(&mut md5_instance)
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&($input_str));
        let result = hasher.finalize();
        format!("{:0>2x}", result)
    }};
}

#[macro_export]
macro_rules! calc_md5_uppercase {
    ($input_str: expr) => {{
        // let mut md5_instance = crypto::md5::Md5::new();
        // crypto::digest::Digest::input_str(&mut md5_instance, &($input_str));
        // crypto::digest::Digest::result_str(&mut md5_instance).to_ascii_uppercase()
        use md5::{Digest, Md5};
        let mut hasher = Md5::new();
        hasher.update(&($input_str));
        let result = hasher.finalize();
        format!("{:0>2X}", result)
    }};
}

#[macro_export]
/// Gen binary type gRPC Metadata.
macro_rules! encode_grpc_header_bin {
    ($raw_data:expr) => {{
        let mut buffer = Vec::with_capacity(512);
        prost::Message::encode(&$raw_data, &mut buffer).unwrap();
        tonic::metadata::BinaryMetadataValue::from_bytes(&buffer)
    }};
}

#[macro_export]
/// Parse gRPC Message
///
/// Param: `&[u8]`, type path of struct
macro_rules! parse_grpc_any {
    ($u8:expr, $struct:path) => {{
        let req_grpc_metadata: $struct = prost::Message::decode($u8).unwrap();
        req_grpc_metadata
    }};
}

#[macro_export]
/// 解析 Binary 类型 gRPC MetadataValue.
macro_rules! parse_grpc_header_bin {
    ($struct:path, $base64_value:expr) => {{
        let req_grpc_metadata_bin = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            $base64_value,
        )
        .unwrap();
        // ? 相对应地可以encode
        let req_grpc_metadata: $struct =
            prost::Message::decode(req_grpc_metadata_bin.as_slice()).unwrap();
        req_grpc_metadata
    }};
    ($bin_name:expr, $struct:path, $request:expr) => {{
        let req_grpc_metadata_bin = $request
            .metadata()
            .get_bin($bin_name)
            .unwrap()
            .as_encoded_bytes();
        let req_grpc_metadata_bin = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD_NO_PAD,
            req_grpc_metadata_bin,
        )
        .unwrap();
        // ? 相对应地可以encode
        let req_grpc_metadata: $struct =
            prost::Message::decode(req_grpc_metadata_bin.as_slice()).unwrap();
        req_grpc_metadata
    }};
}

#[macro_export]
/// Faster way to get current timestamp other than `chrono::Local::now().timestamp()`,
/// 12x faster on my machine.
///
/// # Example
///
/// ```rust
/// use lib_utils::now;
///
/// let now_ts_sec = now!().as_secs(); // Seconds since UNIX_EPOCH
/// let now_ts_millis = now!().as_millis(); // Milliseconds since UNIX_EPOCH
/// ```
///
/// See [`Duration`](https://doc.rust-lang.org/std/time/struct.Duration.html) for more details.
macro_rules! now {
    () => {{
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(t) => t,
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }};
}

#[macro_export]
/// + Gen random string
/// # Example
///
/// ```rust
/// use lib_utils::random_string;
///
/// let rs_1 = random_string!(32); // Use default charset `b"0123456789abcdef"`
/// let rs_2 = random_string!(32, b"0123456789abcdefABCDEF");
/// ```
macro_rules! random_string {
    ($range: expr, $charset: expr) => {{
        let mut rng = rand::thread_rng();
        (0..$range)
            .map(|_| {
                let idx = rand::Rng::gen_range(&mut rng, 0..$charset.len());
                $charset[idx] as char
            })
            .collect::<String>()
    }};
    ($range: expr) => {{
        const CHARSET: &[u8] = b"0123456789abcdef";
        let mut rng = rand::thread_rng();
        (0..$range)
            .map(|_| {
                let idx = rand::Rng::gen_range(&mut rng, 0..16);
                CHARSET[idx] as char
            })
            .collect::<String>()
    }};
}

#[macro_export]
/// + Generates a random string by choosing ones from given candidates.
///
/// Candidates should be `Vec<&str>` or `[&'a str]`.
///
/// # Examples
///
/// ```
/// use lib_utils::random_choice;
///
/// static DIGHT_MAP: [&'static str; 17] = [
/// "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "A", "B", "C", "D", "E", "F", "10",
/// ];
///
/// let rc_1 = random_choice!(32, DIGHT_MAP);
/// let rc_2 = random_choice!(8, 4, 4, 4, 12; "-"; DIGHT_MAP); // like `8310B0E0A-40105-9EC3-8298-36C75D10FEA59`
/// ```
macro_rules! random_choice {
    ($range: expr, $choice_set: expr) => {{
        let mut rng = rand::thread_rng();
        let mut result = String::with_capacity(32);
        (0..$range).for_each(|_| {
            result.push_str($choice_set[rand::Rng::gen_range(&mut rng, 0..$choice_set.len())]);
        });
        result
    }};
    ($($range: expr),+; $split: expr; $choice_set: expr) => {{
        let mut rng = rand::thread_rng();
        let mut result = String::with_capacity(32);
        $(
            (0..$range).for_each(|_| {
                result.push_str($choice_set[rand::Rng::gen_range(&mut rng, 0..$choice_set.len())]);
            });
            result.push_str($split);
        )+
        result.truncate(result.len() - $split.len());
        result
    }};
}
