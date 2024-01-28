use anyhow::{anyhow, Result};
use std::fmt::Write;
use tonic::metadata::MetadataMap;
use tracing::warn;

use crate::error::HeaderError;
use crate::{b64_decode, b64_encode, encode_grpc_header_bin, now, random_string, str_concat};

/// Grpc Metadata or RESTful API Request Header Definition
#[derive(Debug, Clone, Copy)]
pub enum HeaderKey {
    /// `env`, default `prod`
    Env,
    /// `app-key`, like `android` or `android64`
    ///
    /// Need more investigation...
    AppkeyName,
    /// user_agent
    UserAgent,
    /// `x-bili-trace-id`
    BiliTraceId,
    /// `x-bili-aurora-eid`, generated with user's mid
    BiliAuroraEid,
    /// `x-bili-mid`, leave empty if not logged in
    BiliMid,
    /// `x-bili-aurora-zone`, default empty in most time
    BiliAuroraZone,
    /// `x-bili-ticket`
    BiliTicket,
    /// `buvid`
    Buvid,
    /// `x-bili-gaia-vtoken`, default empty in most time
    BiliGaiaVtoken,
    /// `bili-http-engine`, default `cronet`
    BiliHttpEngine,
    // ----------------------------------------------------------------
    // The following headers are only used in gRPC
    // ----------------------------------------------------------------
    /// `fp_local`, device fingerprint locally generated
    FpLocal,
    /// `fp_remote`, device fingerprint remotely stored
    FpRemote,
    /// `session_id`, 8 dight random string
    SessionId,
    // ----------------------------------------------------------------
    // The following headers are only used in gRPC
    // ----------------------------------------------------------------
    /// `authorization`, like `identify_v1 {access_key}`, gRPC Metadata
    Authorization,
    /// `x-bili-metadata-bin`, gRPC Metadata
    BiliMetadataBin,
    /// `x-bili-device-bin`, gRPC Metadata
    BiliDeviceBin,
    /// `x-bili-network-bin`, gRPC Metadata
    BiliNetworkBin,
    /// `x-bili-restriction-bin`, gRPC Metadata
    BiliRestrictionBin,
    /// `x-bili-locale-bin`, gRPC Metadata
    BiliLocaleBin,
    /// `x-bili-exps-bin`, gRPC Metadata
    BiliExpsBin,
    /// `x-bili-fawkes-req-bin`, gRPC Metadata
    BiliFawkesReqBin,
    /// Custom one embedded in codes
    Custom(&'static str),
}

#[allow(dead_code)]
impl HeaderKey {
    #[inline]
    const fn str(self) -> &'static str {
        match self {
            Self::Env => "env",
            Self::AppkeyName => "app-key",
            Self::UserAgent => "user-agent",
            Self::BiliTraceId => "x-bili-trace-id",
            Self::BiliAuroraEid => "x-bili-aurora-eid",
            Self::BiliMid => "x-bili-mid",
            Self::BiliAuroraZone => "x-bili-aurora-zone",
            Self::BiliTicket => "x-bili-ticket",
            Self::Buvid => "buvid",
            Self::BiliGaiaVtoken => "x-bili-gaia-vtoken",
            Self::BiliHttpEngine => "bili-http-engine",
            Self::FpLocal => "fp_local",
            Self::FpRemote => "fp_remote",
            Self::SessionId => "session_id",
            Self::Authorization => "authorization",
            Self::BiliMetadataBin => "x-bili-metadata-bin",
            Self::BiliDeviceBin => "x-bili-device-bin",
            Self::BiliNetworkBin => "x-bili-network-bin",
            Self::BiliRestrictionBin => "x-bili-restriction-bin",
            Self::BiliLocaleBin => "x-bili-locale-bin",
            Self::BiliExpsBin => "x-bili-exps-bin",
            Self::BiliFawkesReqBin => "x-bili-fawkes-req-bin",
            Self::Custom(s) => s,
        }
    }
    #[inline]
    pub const fn default_value(self) -> &'static str {
        match self {
            Self::Env => "prod",
            Self::BiliHttpEngine => "cronet",
            // Network { r#type: Wifi, tf: TfUnknown, oid: "" }
            Self::BiliNetworkBin => "CAE",
            // Locale {
            //    c_locale: Some(LocaleIds { language: "zh", script: "Hans", region: "CN" }),
            //    s_locale: Some(LocaleIds { language: "zh", script: "Hans", region: "CN" }),
            //    sim_code: "", timezone: ""
            // }
            Self::BiliLocaleBin => "Cg4KAnpoEgRIYW5zGgJDThIOCgJ6aBIESGFucxoCQ04",
            _ => "",
        }
    }
}

use lib_bilibili::bapis::metadata::{
    device::Device, fawkes::FawkesReq, locale::Locale, network::Network, parabox::Exps,
    restriction::Restriction, Metadata,
};

pub trait BiliHeaderT {
    fn set(&mut self, key: HeaderKey, value: impl TryInto<http::HeaderValue>) -> &mut Self;
    fn set_binary(&mut self, key: HeaderKey, value: impl AsRef<[u8]>) -> &mut Self;
    /// Set `authorization`
    fn set_access_key(&mut self, access_key: &str) -> &mut Self {
        self.set(
            HeaderKey::Authorization,
            &str_concat!("identify_v1 {}", access_key),
        )
    }
    /// Set `app-key`
    fn set_appkey_name(&mut self, appkey_name: &str) -> &mut Self {
        self.set(HeaderKey::AppkeyName, appkey_name)
    }
    /// Set `buvid`
    fn set_buvid(&mut self, buvid: &str) -> &mut Self {
        self.set(HeaderKey::Buvid, buvid)
    }
    /// Set `fp_local`, `fp_remote`.
    ///
    /// TODO: If `android_id` or `drm_id` is given, `fp_local`, `fp_remote` and `buvid` will be generated and set.
    /// Highly recommend that `drm_id` be given.
    fn set_fp(&mut self, fp: &str, _android_id: Option<&str>, _drm_id: Option<&str>) -> &mut Self {
        // let fp_local = str_concat!(fp, ":", drm_id.unwrap_or_default());
        // let fp_remote = str_concat!(fp, ":", android_id.unwrap_or_default());
        // let buvid = str_concat!(fp, ":", drm_id.unwrap_or_default(), ":", android_id.unwrap_or_default());

        self.set(HeaderKey::FpLocal, fp)
            .set(HeaderKey::FpRemote, fp)
        // .set(HeaderKey::Buvid, &buvid)
    }
    /// Set `x-bili-mid`, `x-bili-aurora-eid`
    fn set_mid(&mut self, mid: u64) -> &mut Self {
        self.set(HeaderKey::BiliMid, &mid.to_string())
            .set(HeaderKey::BiliAuroraEid, &gen_aurora_eid(mid).unwrap())
    }
    /// Set `x-bili-metadata-bin`
    fn set_metadata_bin(&mut self, metadata: Metadata) -> &mut Self {
        self.set_binary(
            HeaderKey::BiliMetadataBin,
            encode_grpc_header_bin!(metadata),
        )
    }
    /// Set `x-bili-device-bin`
    fn set_device_bin(&mut self, device: Device) -> &mut Self {
        self.set_binary(HeaderKey::BiliDeviceBin, encode_grpc_header_bin!(device))
    }
    /// Set `x-bili-network-bin`
    fn set_network_bin(&mut self, network: Network) -> &mut Self {
        self.set_binary(HeaderKey::BiliNetworkBin, encode_grpc_header_bin!(network))
    }
    /// Set `x-bili-restriction-bin`
    fn set_restriction_bin(&mut self, restriction: Restriction) -> &mut Self {
        self.set_binary(
            HeaderKey::BiliRestrictionBin,
            encode_grpc_header_bin!(restriction),
        )
    }
    /// Set `x-bili-locale-bin`
    fn set_locale_bin(&mut self, locale: Locale) -> &mut Self {
        self.set_binary(HeaderKey::BiliLocaleBin, encode_grpc_header_bin!(locale))
    }
    /// Set `x-bili-exps-bin`
    fn set_exps_bin(&mut self, exps: Exps) -> &mut Self {
        self.set_binary(HeaderKey::BiliExpsBin, encode_grpc_header_bin!(exps))
    }
    /// Set `x-bili-fawkes-req-bin`
    fn set_fawkes_req_bin(&mut self, fawkes_req: FawkesReq) -> &mut Self {
        self.set_binary(
            HeaderKey::BiliFawkesReqBin,
            encode_grpc_header_bin!(fawkes_req),
        )
    }
}

/// United `http::HeaderMap` wrapper for both `reqwest` & `tonic`
#[derive(Debug)]
pub struct ManagedHeaderMap {
    inner: http::HeaderMap,
    // Set if it's gRPC Metadata
    is_metadata: bool,
    // Set if it's for Bilibili
    _for_bili: bool,
}

impl ManagedHeaderMap {
    /// Create a new `ManagedHeaderMap`
    ///
    /// - `is_metadata` is set to `true` if it's for gRPC Metadata.
    /// - `for_bili` is set to `true` if it's for requesting Bilibili, now reserved for future use.
    pub fn new(is_metadata: bool, for_bili: bool) -> Self {
        let mut map = Self {
            inner: http::HeaderMap::with_capacity(32),
            is_metadata,
            _for_bili: for_bili,
        };

        if for_bili {
            map.insert_default(HeaderKey::Env);
            map.insert_default(HeaderKey::BiliAuroraEid);
            map.insert_default(HeaderKey::BiliMid);
            map.insert_default(HeaderKey::BiliAuroraZone);
            map.insert_default(HeaderKey::BiliGaiaVtoken);
            map.insert_default(HeaderKey::BiliHttpEngine);

            map.insert(HeaderKey::BiliTraceId, gen_trace_id());
            map.insert(HeaderKey::SessionId, random_string!(8));

            if is_metadata {
                map.insert_default(HeaderKey::BiliNetworkBin);
                map.insert_default(HeaderKey::BiliRestrictionBin);
                map.insert_default(HeaderKey::BiliLocaleBin);
                map.insert_default(HeaderKey::BiliExpsBin);

                // Will be used to replace original tonic Metadata in interceptor.
                map.insert_from_static(HeaderKey::Custom("te"), "trailers");
                map.insert_from_static(HeaderKey::Custom("content-type"), "application/grpc");
            }
        }

        map
    }

    /// Create a new `ManagedHeaderMap` from existing `http::HeaderMap`
    ///
    /// # WARNING
    ///
    /// Just simply converts the given `http::HeaderMap` to `ManagedHeaderMap`,will
    /// **NOT** check if the given `http::HeaderMap` is valid for gRPC Metadata, or
    /// adding any default headers like `.new()`, which may cause runtime panics when
    /// `.take_inner()`.
    pub fn new_from_existing(inner: http::HeaderMap, is_metadata: bool, for_bili: bool) -> Self {
        Self {
            inner,
            is_metadata,
            _for_bili: for_bili,
        }
    }

    #[inline]
    /// Get gRPC ascii type Metadata or general http header
    pub fn get(&self, key: HeaderKey) -> Result<&str> {
        self.inner
            .get(key.str())
            .ok_or(HeaderError::KeyNotExist(key.str().to_owned()))?
            .to_str()
            .map_err(|e| anyhow!(HeaderError::from(e)))
    }

    #[inline]
    /// Get gRPC Binary type Metadata
    ///
    /// # Panics(debug)
    ///
    /// This function panics if the argument `key` is not a valid binary type gRPC Metadata
    /// key when `self.is_metadata`.
    pub fn get_bin(&self, key: HeaderKey) -> Result<Vec<u8>> {
        debug_assert!(
            self.is_metadata && Self::is_valid_bin_key(key.str()),
            "Not a gRPC Metadata or key [{}] is not valid binary type gRPC Metadata key",
            key.str()
        );

        let b64_str = self.get(key)?;

        b64_decode!(b64_str, base64::engine::general_purpose::STANDARD_NO_PAD).map_err(|e| {
            anyhow!(HeaderError::Base64DecodeError {
                key: key.str().to_owned(),
                value: b64_str.to_owned(),
                e,
            })
        })
    }

    /// Insert gRPC ascii type Metadata or general http header
    ///
    /// # Panics(debug)
    ///
    /// This function panics if the argument `key` is not a valid ascii type gRPC Metadata
    /// key when `self.is_metadata`.
    pub fn insert<T>(&mut self, key: HeaderKey, value: T) -> &mut Self
    where
        T: TryInto<http::HeaderValue>,
    {
        debug_assert!(!self.is_metadata || (!Self::is_valid_bin_key(key.str())));

        let value = match value.try_into() {
            Ok(value) => value,
            Err(_) => {
                warn!(
                    "Given value of [{}] cannot be converted to http::HeaderValue, use default value instead",
                    key.str()
                );
                // SAFE: Default value must be valid http header value
                key.default_value().parse().unwrap()
            }
        };
        self.inner.insert(key.str(), value);
        self
    }

    /// Insert gRPC Binary type Metadata
    ///
    /// # Panics(debug)
    ///
    /// This function panics if the argument `key` is not a valid binary type gRPC Metadata
    /// key when `self.is_metadata`.
    pub fn insert_bin(&mut self, key: HeaderKey, value: impl AsRef<[u8]>) -> &mut Self {
        debug_assert!(
            self.is_metadata && Self::is_valid_bin_key(key.str()),
            "Not a gRPC Metadata or key [{}] is not valid binary type gRPC Metadata key",
            key.str()
        );

        let data_base64 = b64_encode!(value, base64::engine::general_purpose::STANDARD_NO_PAD);

        // SAFE: Base64 encoded data value must be valid http header value
        self.inner.insert(key.str(), data_base64.parse().unwrap());

        self
    }

    #[inline]
    /// Insert gRPC ascii type Metadata or general http header from static string
    ///
    /// # Panics
    ///
    /// This function panics if the argument `value` contains invalid header value characters.
    pub fn insert_from_static(&mut self, key: HeaderKey, value: &'static str) -> &mut Self {
        self.inner
            .insert(key.str(), http::HeaderValue::from_static(value));

        self
    }

    #[inline]
    fn insert_default(&mut self, key: HeaderKey) -> &mut Self {
        self.inner.insert(
            key.str(),
            http::HeaderValue::from_static(key.default_value()),
        );

        self
    }

    #[inline]
    /// Take inner `http::HeaderMap` out
    ///
    /// **DO NOT** use original builder after calling this, as inner data has been taken out
    pub fn take_inner(&mut self) -> http::HeaderMap {
        // Should not panic when release
        self.verify();

        std::mem::take(&mut self.inner)
    }

    /// Verify if all required headers are set
    ///
    /// # Panics(debug)
    ///
    /// Not setting any possible header will cause runtime panics.
    fn verify(&self) {
        debug_assert!(self.get(HeaderKey::Env).is_ok());
        debug_assert!(self.get(HeaderKey::AppkeyName).is_ok());
        // Always should not forget to set `user-agent`
        assert!(self.get(HeaderKey::UserAgent).is_ok());
        debug_assert!(self.get(HeaderKey::BiliTraceId).is_ok());
        debug_assert!(self.get(HeaderKey::BiliAuroraEid).is_ok());
        debug_assert!(self.get(HeaderKey::BiliMid).is_ok());
        debug_assert!(self.get(HeaderKey::BiliAuroraZone).is_ok());
        debug_assert!(self.get(HeaderKey::BiliTicket).is_ok());
        debug_assert!(self.get(HeaderKey::Buvid).is_ok());
        debug_assert!(self.get(HeaderKey::BiliGaiaVtoken).is_ok());
        debug_assert!(self.get(HeaderKey::BiliHttpEngine).is_ok());
        debug_assert!(self.get(HeaderKey::FpLocal).is_ok());
        // debug_assert!(self.get(HeaderKey::FpRemote).is_ok());
        debug_assert!(self.get(HeaderKey::SessionId).is_ok());
        // Only when logged in `authorization` should be set
        // debug_assert!(self.is_metadata && self.get(HeaderKey::Authorization).is_ok());
        // Always should not forget to set `x-bili-metadata-bin`
        assert!(self.is_metadata && self.get_bin(HeaderKey::BiliMetadataBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliDeviceBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliNetworkBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliRestrictionBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliLocaleBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliExpsBin).is_ok());
        debug_assert!(self.is_metadata && self.get_bin(HeaderKey::BiliFawkesReqBin).is_ok());
    }

    #[inline]
    /// Check if binary type gRPC Metadata key is valid
    fn is_valid_bin_key(key: &str) -> bool {
        key.ends_with("-bin")
    }
}

impl BiliHeaderT for ManagedHeaderMap {
    #[inline]
    fn set(&mut self, key: HeaderKey, value: impl TryInto<http::HeaderValue>) -> &mut Self {
        self.insert(key, value)
    }

    #[inline]
    fn set_binary(&mut self, key: HeaderKey, value: impl AsRef<[u8]>) -> &mut Self {
        self.insert_bin(key, value)
    }
}

impl tonic::service::Interceptor for ManagedHeaderMap {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let (_, extensions, message) = request.into_parts();
        let metadata = MetadataMap::from_headers(self.take_inner());
        Ok(tonic::Request::from_parts(metadata, extensions, message))
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
        warn!("UID is 0, eid will be None");
        return None;
    }
    let result_byte = eid_xor(uid.to_string());
    Some(base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        result_byte,
    ))
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

#[cfg(test)]
mod test {
    use crate::parse_grpc_header_bin;

    #[test]
    fn test() {
        let d = parse_grpc_header_bin!(
            lib_bilibili::bapis::metadata::locale::Locale,
            "Cg4KAnpoEgRIYW5zGgJDThIOCgJ6aBIESGFucxoCQ04"
        );
        println!("{:?}", d);
    }

    #[test]
    fn t() {
        use super::ManagedHeaderMap;

        let mut headers = ManagedHeaderMap::new(false, false);
        headers.insert(super::HeaderKey::AppkeyName, 6);
        headers.insert(super::HeaderKey::Authorization, "");

        println!("{:?}", headers.take_inner())
    }
}
