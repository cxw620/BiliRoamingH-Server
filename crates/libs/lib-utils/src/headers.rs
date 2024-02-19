use anyhow::{anyhow, Result};
use http_02::{HeaderMap as HttpHeaderMap, HeaderValue as HttpHeaderValue};
use tonic::metadata::MetadataMap;

use crate::{
    b64_decode, encode_grpc_header_bin,
    error::HeaderError,
    misc::{gen_aurora_eid, gen_trace_id},
    random_string, str_concat,
};

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
    fn set(&mut self, key: HeaderKey, value: impl TryInto<HttpHeaderValue>) -> &mut Self;
    fn set_binary(&mut self, key: HeaderKey, value: impl AsRef<[u8]> + 'static) -> &mut Self;
    /// Set `authorization`
    fn set_access_key(&mut self, access_key: &str) -> &mut Self {
        self.set(
            HeaderKey::Authorization,
            &str_concat!("identify_v1 ", access_key),
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
    /// Set `x-bili-ticket` if given
    fn set_ticket(&mut self, ticket: Option<&str>) -> &mut Self {
        if let Some(ticket) = ticket {
            self.set(HeaderKey::BiliTicket, ticket)
        } else {
            self
        }
    }
    /// Set `user-agent`, if given None then use default
    fn set_user_agent(&mut self, user_agent: Option<&str>) -> &mut Self {
        self.set(
            HeaderKey::UserAgent,
            user_agent.unwrap_or(user_agent::FakeUA::UA_APP_DEFAULT),
        )
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
#[derive(Debug, Clone)]
pub struct ManagedHeaderMap {
    inner: HttpHeaderMap,
    // Set if it's gRPC Metadata
    is_metadata: bool,
    // Set if it's for Bilibili
    for_bili: bool,
}

impl ManagedHeaderMap {
    /// Create a new `ManagedHeaderMap`
    ///
    /// - `is_metadata` is set to `true` if it's for gRPC Metadata.
    /// - `for_bili` is set to `true` if it's for requesting Bilibili, now reserved for future use.
    pub fn new(is_metadata: bool, for_bili: bool) -> Self {
        let mut map = Self {
            inner: HttpHeaderMap::with_capacity(32),
            is_metadata,
            for_bili,
        };

        if is_metadata {
            // Will be used to replace original tonic Metadata in interceptor.
            map.insert_from_static(HeaderKey::Custom("te"), "trailers");
            map.insert_from_static(HeaderKey::Custom("content-type"), "application/grpc");
        }

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
            }
        } else {
            // Add basic headers for general http request
            map.insert_from_static(HeaderKey::UserAgent, user_agent::FakeUA::UA_DALVIK_DEFAULT);
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
    pub fn new_from_existing(inner: HttpHeaderMap, is_metadata: bool, for_bili: bool) -> Self {
        Self {
            inner,
            is_metadata,
            for_bili,
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

    #[inline]
    /// Check if key exist
    pub fn contains_key(&self, key: HeaderKey) -> bool {
        self.inner.contains_key(key.str())
    }

    /// Insert gRPC ascii type Metadata or general http header
    ///
    /// # Panics(debug)
    ///
    /// This function panics if the argument `key` is not a valid ascii type gRPC Metadata
    /// key when `self.is_metadata`.
    pub fn insert<T>(&mut self, key: HeaderKey, value: T) -> &mut Self
    where
        T: TryInto<HttpHeaderValue>,
    {
        debug_assert!(!self.is_metadata || (!Self::is_valid_bin_key(key.str())));

        let value = match value.try_into() {
            Ok(value) => value,
            Err(_) => {
                tracing::warn!(
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
    pub fn insert_bin(&mut self, key: HeaderKey, value: impl AsRef<[u8]> + 'static) -> &mut Self {
        debug_assert!(
            self.is_metadata && Self::is_valid_bin_key(key.str()),
            "Not a gRPC Metadata or key [{}] is not valid binary type gRPC Metadata key",
            key.str()
        );

        // SAFE: Base64 encoded data value must be valid http header value
        let value = unsafe { HttpHeaderValue::from_maybe_shared_unchecked(value) };
        self.inner.insert(key.str(), value);

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
            .insert(key.str(), HttpHeaderValue::from_static(value));

        self
    }

    #[inline]
    fn insert_default(&mut self, key: HeaderKey) -> &mut Self {
        self.inner
            .insert(key.str(), HttpHeaderValue::from_static(key.default_value()));

        self
    }

    #[inline]
    /// Take inner `http::HeaderMap` out
    ///
    /// **DO NOT** use original builder after calling this, as inner data has been taken out
    pub fn take_inner(&mut self) -> HttpHeaderMap {
        // Should not panic when release
        self.verify();

        std::mem::take(&mut self.inner)
    }

    /// Verify if all required headers are set.
    ///
    /// Will skip verification if `self.for_bili` is `false`.
    ///
    /// # Panics(debug)
    ///
    /// Not setting any possible header will cause runtime panics.
    fn verify(&self) {
        if !self.for_bili {
            return;
        }
        macro_rules! check_if_exist {
            ($key:expr) => {
                if (!self.contains_key($key)) {
                    tracing::warn!("Header [{}] is not set", $key.str())
                }
            };
            ($precondition:expr, $key:expr) => {
                if ($precondition && !self.contains_key($key)) {
                    tracing::warn!("Header [{}] is not set", $key.str())
                }
            };
        }
        macro_rules! check_if_exist_or_panic {
            ($key:expr) => {
                if !self.contains_key($key) {
                    if cfg!(debug_assertions) {
                        panic!("Header [{}] is not set", $key.str())
                    } else {
                        tracing::warn!("Header [{}] is not set", $key.str())
                    }
                }
            };
            ($precondition:expr, $key:expr) => {
                if ($precondition && !self.contains_key($key)) {
                    if cfg!(debug_assertions) {
                        panic!("Header [{}] is not set", $key.str())
                    } else {
                        tracing::warn!("Header [{}] is not set", $key.str())
                    }
                }
            };
        }

        check_if_exist!(HeaderKey::Env);
        check_if_exist!(HeaderKey::AppkeyName);
        check_if_exist_or_panic!(HeaderKey::UserAgent);
        check_if_exist!(HeaderKey::BiliTraceId);
        check_if_exist!(HeaderKey::BiliAuroraEid);
        check_if_exist!(HeaderKey::BiliMid);
        check_if_exist!(HeaderKey::BiliAuroraZone);
        check_if_exist!(HeaderKey::BiliTicket);
        check_if_exist!(HeaderKey::Buvid);
        check_if_exist!(HeaderKey::BiliGaiaVtoken);
        check_if_exist!(HeaderKey::BiliHttpEngine);
        check_if_exist!(HeaderKey::FpLocal);
        check_if_exist!(HeaderKey::FpRemote);
        check_if_exist!(HeaderKey::SessionId);
        check_if_exist!(HeaderKey::Authorization);
        check_if_exist!(self.is_metadata, HeaderKey::BiliMetadataBin);
        check_if_exist_or_panic!(self.is_metadata, HeaderKey::BiliDeviceBin);
        check_if_exist!(self.is_metadata, HeaderKey::BiliNetworkBin);
        check_if_exist!(self.is_metadata, HeaderKey::BiliRestrictionBin);
        check_if_exist!(self.is_metadata, HeaderKey::BiliLocaleBin);
        check_if_exist!(self.is_metadata, HeaderKey::BiliExpsBin);
        check_if_exist!(self.is_metadata, HeaderKey::BiliFawkesReqBin);
    }

    #[inline]
    /// Check if binary type gRPC Metadata key is valid
    fn is_valid_bin_key(key: &str) -> bool {
        key.ends_with("-bin")
    }
}

impl BiliHeaderT for ManagedHeaderMap {
    #[inline]
    fn set(&mut self, key: HeaderKey, value: impl TryInto<HttpHeaderValue>) -> &mut Self {
        self.insert(key, value)
    }

    #[inline]
    fn set_binary(&mut self, key: HeaderKey, value: impl AsRef<[u8]> + 'static) -> &mut Self {
        self.insert_bin(key, value)
    }
}

impl Into<HttpHeaderMap> for ManagedHeaderMap {
    fn into(mut self) -> HttpHeaderMap {
        self.take_inner()
    }
}

impl tonic::service::Interceptor for ManagedHeaderMap {
    fn call(&mut self, request: tonic::Request<()>) -> Result<tonic::Request<()>, tonic::Status> {
        let (metadata_original, extensions, message) = request.into_parts();
        let header_map_original = metadata_original.into_headers();

        let mut header_map = self.take_inner();
        if let Some(encoding) = header_map_original.get("grpc-encoding") {
            header_map.insert("grpc-encoding", encoding.clone());
        }
        if let Some(accept_encoding) = header_map_original.get("grpc-accept-encoding") {
            header_map.insert("grpc-accept-encoding", accept_encoding.clone());
        }

        Ok(tonic::Request::from_parts(
            MetadataMap::from_headers(header_map),
            extensions,
            message,
        ))
    }
}

impl Into<tonic::metadata::MetadataMap> for ManagedHeaderMap {
    fn into(mut self) -> tonic::metadata::MetadataMap {
        MetadataMap::from_headers(self.take_inner())
    }
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

#[allow(dead_code)]
pub(crate) mod user_agent {
    use crate::str_concat;
    use rand::Rng;
    use std::fmt::Write;

    pub trait TUserAgent {
        fn web(&self) -> String;
        fn mobile(&self) -> String;
        fn dalvik(&self) -> String;
        fn bili_app(&self) -> String;
    }

    pub struct UniteUA {
        app_build: String,
        app_ver: String,
        mobi_app: String,
        os_ver: String,
        network: String,
        ua_device_model: String,
        ua_device_build: String,
    }

    impl Default for UniteUA {
        fn default() -> Self {
            Self {
                app_build: FakeUA::APP_BUILD_DEFAULT.to_owned(),
                app_ver: FakeUA::APP_VER_DEFAULT.to_owned(),
                mobi_app: FakeUA::MOBI_APP_DEFAULT.to_owned(),
                os_ver: FakeUA::OS_VER_DEFAULT.to_owned(),
                network: FakeUA::NETWORK_DEFAULT.to_owned(),
                ua_device_model: FakeUA::DEVICE_MODEL_DEFAULT.to_owned(),
                ua_device_build: FakeUA::DEVICE_BUILD_DEFAULT.to_owned(),
            }
        }
    }

    impl TUserAgent for UniteUA {
        fn web(&self) -> String {
            FakeUA::UA_WEB_DEFAULT.to_owned()
        }
        fn mobile(&self) -> String {
            str_concat!(
                "Mozilla/5.0 (Linux; U; Android ",
                &self.os_ver,
                "; ",
                &self.ua_device_model,
                " Build/",
                &self.ua_device_build,
                ") AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Mobile Safari/537.36"
            )
        }
        fn dalvik(&self) -> String {
            str_concat!(
                "Dalvik/2.1.0 (Linux; U; Android ",
                &self.os_ver,
                "; ",
                &self.ua_device_model,
                " Build/",
                &self.ua_device_build,
                ") ",
                &self.app_ver,
                " os/android model/",
                &self.ua_device_model,
                " mobi_app/",
                &self.mobi_app,
                " build/",
                &self.app_build,
                " channel/master innerVer/",
                &self.app_build,
                " osVer/",
                &self.os_ver,
                " network/",
                &self.network
            )
        }
        fn bili_app(&self) -> String {
            str_concat!(
                "Mozilla/5.0 BiliDroid/",
                &self.app_ver,
                " (bbcallen@gmail.com) os/android model/",
                &self.ua_device_model,
                " mobi_app/",
                &self.mobi_app,
                " build/",
                &self.app_build,
                " channel/master innerVer/",
                &self.app_build,
                " osVer/",
                &self.os_ver,
                " network/",
                &self.network
            )
        }
    }

    pub struct UniteUABuilder {
        inner: UniteUA,
    }

    impl Default for UniteUABuilder {
        fn default() -> Self {
            Self {
                inner: UniteUA::default(),
            }
        }
    }

    #[allow(dead_code)]
    impl UniteUABuilder {
        pub fn new() -> Self {
            Self::default()
        }
        /// Generate a random UA
        pub fn gen_fake_ua(&mut self) -> &mut Self {
            let (os_ver, device_model, device_build) = FakeUA::gen_random_phone();
            self.inner.os_ver = os_ver.to_owned();
            self.inner.ua_device_model = device_model.to_owned();
            self.inner.ua_device_build = device_build.to_owned();
            self
        }
        /// Set Bilibili APP Build, will **also** set APP Version
        pub fn set_app_build(&mut self, app_build: impl Into<String>) -> &mut Self {
            let app_build = app_build.into();
            if app_build.len() != 7 {
                tracing::error!(target: "UniteUA", "Invalid Bilibili APP Build: {}", app_build);
                return self;
            }
            let mut app_ver = String::with_capacity(16);
            write!(
                app_ver,
                "{}.{}.{}",
                &app_build[0..1],
                app_build[1..3].parse::<u8>().unwrap_or(38),
                &app_build[3..4],
            )
            .unwrap();
            self.inner.app_build = app_build;
            self.inner.app_ver = app_ver;
            self
        }

        /// Set Bilibili APP Version, will **also** set APP Build
        pub fn set_app_ver(&mut self, app_ver: impl Into<String>) -> &mut Self {
            let app_ver: String = app_ver.into();
            let app_build_vec: Vec<&str> = app_ver.split(".").collect();
            if app_build_vec.len() < 3 {
                tracing::error!(target: "UniteUA", "Invalid Bilibili APP Version: {}", app_ver);
                return self;
            }
            let mut app_build = String::with_capacity(16);
            write!(
                app_build,
                "{}{:02}{}300",
                app_build_vec[0],
                app_build_vec[1].parse::<u8>().unwrap_or(38),
                app_build_vec[2],
            )
            .unwrap();
            self.inner.app_build = app_build;
            self.inner.app_ver = app_ver;
            self
        }
        /// Set Bilibili mobi_app
        pub fn set_mobi_app(&mut self, mobi_app: impl Into<String>) -> &mut Self {
            self.inner.mobi_app = mobi_app.into();
            self
        }
        /// Set Device OS Version
        pub fn set_os_ver(&mut self, os_ver: impl Into<String>) -> &mut Self {
            self.inner.os_ver = os_ver.into();
            self
        }
        /// Set Device Network
        pub fn set_network(&mut self, network: impl Into<String>) -> &mut Self {
            self.inner.network = network.into();
            self
        }
        /// Set Device Model
        pub fn set_device_model(&mut self, device_model: impl Into<String>) -> &mut Self {
            self.inner.ua_device_model = device_model.into();
            self
        }
        /// Set Device Build
        pub fn set_device_build(&mut self, device_build: impl Into<String>) -> &mut Self {
            self.inner.ua_device_build = device_build.into();
            self
        }

        pub fn build(&mut self) -> UniteUA {
            std::mem::take(&mut self.inner)
        }
    }

    impl From<UniteUA> for UniteUABuilder {
        fn from(value: UniteUA) -> Self {
            Self { inner: value }
        }
    }

    pub enum FakeUA {
        Web,                                 // 网页版, 统一使用 Chrome
        Mobile,                              // 移动UA, 移动版 Chrome
        Dalvik,                              //App的UA, Dalvik 开头的类型
        BiliApp(&'static str, &'static str), //Bilibili 的 UA, 类似 Mozilla/5.0 BiliDroid/{6.80.0}{ (bbcallen@gmail.com) os/android model/M2012K11AC mobi_app/android build/6800300 channel/master innerVer/6800310 osVer/12 network/2
    }

    impl FakeUA {
        pub const DEVICE_MODEL_DEFAULT: &'static str = "NOH-AN01";
        pub const DEVICE_BUILD_DEFAULT: &'static str = "HUAWEINOH-AN01";
        pub const APP_VER_DEFAULT: &'static str = "7.38.0";
        pub const MOBI_APP_DEFAULT: &'static str = "android";
        pub const APP_BUILD_DEFAULT: &'static str = "7380300";
        pub const OS_VER_DEFAULT: &'static str = "12";
        pub const NETWORK_DEFAULT: &'static str = "2";
        pub const UA_WEB_DEFAULT: &'static str = "Mozilla/5.0 (Windows NT 10.0; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Safari/537.36";
        pub const UA_MOBILE_DEFAULT: &'static str = "Mozilla/5.0 (Linux; U; Android 12; NOH-AN01 Build/HUAWEINOH-AN01) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/114.0.0.0 Mobile Safari/537.36";
        pub const UA_DALVIK_DEFAULT: &'static str = "Dalvik/2.1.0 (Linux; U; Android 12; NOH-AN01 Build/HUAWEINOH-AN01) 7.38.0 os/android model/NOH-AN01 mobi_app/android build/7380300 channel/master innerVer/7380300 osVer/12 network/2";
        pub const UA_APP_DEFAULT: &'static str = "Mozilla/5.0 BiliDroid/7.38.0 (bbcallen@gmail.com) os/android model/NOH-AN01 mobi_app/android build/7380300 channel/master innerVer/7380310 osVer/12 network/2";

        #[inline]
        fn gen_random_phone() -> (&'static str, &'static str, &'static str) {
            let phones = [
                ("13", "Pixel 6 Pro", "TQ1A.221205.011"),
                ("13", "SM-S9080", "TP1A.220624.014"),
                ("13", "2201122C", "TKQ1.220807.001"),
                ("12", "JEF-AN00", "HUAWEIJEF-AN00"),
                ("12", "VOG-AL10", "HUAWEIVOG-AL10"),
                ("12", "ELS-AN00", "HUAWEIELS-AN00"),
                ("12", "NOH-AN01", "HUAWEINOH-AN01"),
                ("11", "SKW-A0", "SKYW2203210CN00MR1"),
                ("11", "21091116AC", "RP1A.200720.011"),
                ("10", "VOG-AL10", "HUAWEIVOG-AL10"),
                ("10", "JEF-AN00", "HUAWEIJEF-AN00"),
                ("10", "VOG-AL10", "HUAWEIVOG-AL10"),
                ("10", "ELS-AN00", "HUAWEIELS-AN00"),
                ("9", "BND-AL10", "HONORBND-AL10"),
                ("9", "ALP-AL00", "HUAWEIALP-AL00"),
            ];
            phones[rand::thread_rng().gen_range(0..=14)]
        }
    }
}
