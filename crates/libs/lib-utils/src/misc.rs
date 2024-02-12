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
