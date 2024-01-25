use crate::{calc_md5, str_concat};
use anyhow::{bail, Result};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SignErr {
    #[error("Invalid wbi key, length not 64")]
    InvalidWbiKey,
    // #[error("Unknown appkey {0}, corresponding appsec not found")]
    // UnknownAppkey(String),
}

#[derive(Debug)]
pub enum Signer<'s> {
    None,
    Wbi { img_key: &'s str, sub_key: &'s str },
}

/// WBI Sign implementation V1.0.2
///
/// Last updated: 24-01-25 15:20
pub struct Wbi;

impl Wbi {
    #[inline]
    pub fn gen_mixin_key(img_key: &str, sub_key: &str) -> Result<String> {
        let wbi_key = str_concat!(img_key, sub_key);
        if wbi_key.len() != 64 {
            // TODO Request upstream for new one
            bail!(SignErr::InvalidWbiKey);
        }

        const MIXIN_KEY_ENC_TAB: [u8; 64] = [
            46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42,
            19, 29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60,
            51, 30, 4, 22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
        ];

        let wbi_key_bytes = wbi_key.as_bytes();
        let mut mixin_key = {
            let binding = MIXIN_KEY_ENC_TAB
                .iter()
                .map(|n| wbi_key_bytes[*n as usize])
                .collect::<Vec<u8>>();
            // SAFE: `binding` is guaranteed to be valid UTF-8
            unsafe { String::from_utf8_unchecked(binding) }
        };

        mixin_key.truncate(32);

        Ok(mixin_key)
    }

    /// Calculate `w_rid` param value.
    ///
    /// Pass `sorted_params` with `wts` and `mixin_key` to this function.
    #[inline]
    pub fn gen_w_rid(sorted_params: &str, mixin_key: &str) -> String {
        calc_md5!(str_concat!(&sorted_params, &mixin_key))
    }

    // For future use.
    // fn swap_string(input: &str, t: u32) -> String {
    //     if input.len() % 2 != 0 {
    //         return input.to_owned();
    //     }
    //     if t == 0 {
    //         return input.to_owned();
    //     }
    //     if input.len() == 2u32.pow(t) as usize {
    //         return input.chars().rev().collect();
    //     }
    //     let mid = input.len() / 2;
    //     let r = &input[..mid];
    //     let n = &input[mid..];
    //     str_concat!(&Self::swap_string(n, t - 1), &Self::swap_string(r, t - 1))
    // }
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
