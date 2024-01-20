use std::borrow::Cow;

#[derive(Debug)]
pub enum Signer {
    None,
}

impl Signer {
    pub fn sign<'q>(&self, _params: &mut Vec<(&'q str, Cow<'q, str>)>) -> String {
        todo!()
    }
}

type HmacSha256 = hmac::Hmac<sha2::Sha256>;
use hmac::Mac;

#[allow(dead_code)]
enum BiliTicket {
    App {
        /// context, generated with `com.bapis.bilibili.metadata.device.Device`
        device_info: Vec<u8>,
        /// x-fingerprint, generated with `datacenter.hakase.protobuf.AndroidDeviceInfo`
        fingerprint: Vec<u8>,
        /// x-exbadbasket, can leave it empty but should with it
        exbadbasket: Vec<u8>,
    },
    Web {
        ts: u64,
    },
}

#[allow(dead_code)]
impl BiliTicket {
    #[inline]
    const fn hmac_key(&self) -> &'static [u8] {
        match self {
            Self::App { .. } => b"Ezlc3tgtl",
            Self::Web { .. } => b"XgwSnGZ1p",
        }
    }
    pub fn sign(self) -> Vec<u8> {
        let mut mac = HmacSha256::new_from_slice(self.hmac_key()).unwrap();
        match self {
            Self::App {
                device_info,
                fingerprint,
                exbadbasket,
            } => {
                mac.update(&device_info);
                // Originally TreeMap in Java is used, making keys are sequentially sorted.
                mac.update(b"x-exbadbasket");
                mac.update(&exbadbasket);
                mac.update(b"x-fingerprint");
                mac.update(&fingerprint);
            }
            Self::Web { ts } => {
                mac.update(b"ts");
                mac.update(ts.to_string().as_bytes());
            }
        }
        mac.finalize().into_bytes().to_vec()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_app_with_eb() {
        let d = BiliTicket::App {
            device_info: vec![0x1, 0x2, 0x3, 0x4],
            fingerprint: vec![0x5, 0x6, 0x7, 0x8],
            exbadbasket: vec![0x5, 0x6, 0x7, 0x8],
        }
        .sign();
        const EXAMPLE: [u8; 32] = [
            0x4b, 0x77, 0x90, 0xee, 0x46, 0x42, 0x23, 0x54, 0x07, 0x31, 0xd5, 0x0c, 0xbf, 0x71,
            0xd8, 0xa8, 0x62, 0x29, 0x43, 0xae, 0xa6, 0x79, 0xad, 0x12, 0x61, 0x89, 0xf6, 0xb4,
            0x17, 0x67, 0xc5, 0xaa,
        ];
        assert_eq!(d, EXAMPLE);
    }
    // Seems without crate hex support... Damn
    // #[test]
    // fn test_web() {
    //     let d = BiliTicket::Web { ts: 1705658461 }.sign();
    //     assert_eq!(
    //         hex::encode(d).as_str(),
    //         "3c22306bd1ec1227b9d07270c6846a58488b9c554eecdf2a40ae518b25f7c59d"
    //     )
    // }
}
