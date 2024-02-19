use std::fmt::Write;

use crate::{b64_encode, now, random_string, str_concat};

#[derive(Debug)]
pub enum BiliArea {
    CN,
    HKMO,
    TW,
    SEA,
    Unknown,
}

impl Default for BiliArea {
    fn default() -> Self {
        BiliArea::Unknown
    }
}

impl BiliArea {
    pub const fn str(&self) -> &'static str {
        match self {
            BiliArea::CN => "cn",
            BiliArea::HKMO => "hk",
            BiliArea::TW => "tw",
            BiliArea::SEA => "th",
            BiliArea::Unknown => "",
        }
    }
}

impl From<&str> for BiliArea {
    fn from(s: &str) -> Self {
        match s {
            "cn" => BiliArea::CN,
            "hk" => BiliArea::HKMO,
            "tw" => BiliArea::TW,
            "th" => BiliArea::SEA,
            _ => BiliArea::Unknown,
        }
    }
}

#[inline]
/// 合成 `x-bili-trace-id`
pub fn gen_trace_id() -> String {
    // 来自混淆后的tv.danmaku.bili.aurora.api.trace.a
    // 06e903399574695df75be114ff63ac64:f75be114ff63ac64:0:0

    let random_id = random_string!(32);
    let mut random_trace_id = String::with_capacity(40);
    random_trace_id.push_str(&random_id[0..24]);

    let mut b_arr: [i8; 3] = [0i8; 3];
    let mut ts = now!().as_secs() as i128;
    for i in (0..3).rev() {
        ts >>= 8;
        b_arr[i] = {
            // 滑天下之大稽...
            // 应该这样没有问题吧?
            if ((ts / 128) % 2) == 0 {
                (ts % 256) as i8
            } else {
                (ts % 256 - 256) as i8
            }
        }
    }
    for i in 0..3 {
        write!(random_trace_id, "{:0>2x}", b_arr[i]).unwrap_or_default();
    }
    random_trace_id.push_str(&random_id[30..32]);

    str_concat!(&random_trace_id, ":", &random_trace_id[16..32], ":0:0")
}

#[inline]
/// 合成 `x-bili-aurora-eid`
pub fn gen_aurora_eid(uid: u64) -> Option<String> {
    if uid == 0 {
        tracing::warn!("UID is 0, eid will be None");
        return None;
    }
    let result_byte = eid_xor(uid.to_string());
    Some(b64_encode!(result_byte))
}

#[inline]
fn eid_xor(input: impl AsRef<[u8]>) -> Vec<u8> {
    let mut result_byte = Vec::with_capacity(64);
    input
        .as_ref()
        .iter()
        .enumerate()
        .for_each(|(i, v)| result_byte.push(v ^ (b"ad1va46a7lza"[i % 12])));
    result_byte
}
