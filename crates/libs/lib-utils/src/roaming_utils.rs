#[derive(Copy, Clone, Debug, Eq, PartialEq)]
/// BiliRoaming Area
pub enum RoamingArea {
    /// China Mainland
    CN,
    /// Hong Kong or Macau
    HK,
    /// Taiwan
    TW,
    /// South East Asia
    SEA,
    /// Unknown Area
    Unknown,
}

impl RoamingArea {
    #[inline]
    pub const fn as_str(&self) -> &'static str {
        match self {
            RoamingArea::CN => "cn",
            RoamingArea::HK => "hk",
            RoamingArea::TW => "tw",
            RoamingArea::SEA => "th",
            RoamingArea::Unknown => "",
        }
    }
}

impl From<&str> for RoamingArea {
    #[inline]
    fn from(s: &str) -> Self {
        match s {
            "cn" => Self::CN,
            "hk" => Self::HK,
            "tw" => Self::TW,
            "th" => Self::SEA,
            _ => Self::Unknown,
        }
    }
}
