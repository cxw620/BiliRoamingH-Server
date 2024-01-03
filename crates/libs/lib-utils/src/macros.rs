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
