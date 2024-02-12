/// 用户信息
#[derive(Debug)]
pub struct UserInfo {
    pub mid: u64,
    pub vip_info: VipInfo,
}

#[derive(Debug, Default)]
pub struct VipInfo {
    /// 大会员类型
    ///
    /// - 0 无大会员
    /// - 1 月度大会员
    /// - 2 年度大会员
    r#type: i64,
    /// 大会员状态
    ///
    /// - 0 非大会员
    /// - 1 大会员
    /// - 2 大会员(冻结)
    status: i64,
    /// 大会员到期时间(毫秒时间戳)
    pub due_date: i64,
}

impl VipInfo {
    #[inline]
    pub fn is_effective_vip(&self) -> bool {
        (self.r#type == 1 || self.r#type == 2) && self.status == 1
    }

    #[inline]
    pub fn is_frozen_vip(&self) -> bool {
        (self.r#type == 1 || self.r#type == 2) && self.status == 2
    }
}

impl From<x_v2_account_myinfo::AccountInfo> for UserInfo {
    fn from(info: x_v2_account_myinfo::AccountInfo) -> Self {
        Self {
            mid: info.mid,
            vip_info: VipInfo {
                r#type: info.vip.r#type,
                status: info.vip.status,
                due_date: info.vip.due_date,
            },
        }
    }
}

pub mod x_v2_account_myinfo {
    pub static API_HOST: &'static str = "app.bilibili.com";
    pub static API_PATH: &'static str = "/x/v2/account/myinfo";

    use serde::{Deserialize, Serialize};

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    pub struct AccountInfo {
        /// 用户 MID
        pub mid: u64,
        /// 用户昵称
        pub name: String,
        // /// 用户签名
        // pub sign: String,
        // /// 用户硬币数
        // pub coins: f64,
        // /// 用户生日, `yyyy-mm-dd`
        // pub birthday: String,
        // /// 是否已设置生日
        // pub set_birthday: bool,
        /// 用户头像 URL
        pub face: String,
        // /// 用户头像 NFT
        // pub face_nft_new: i64,
        /// 性别?
        ///
        /// - 0 保密
        /// - 1 男
        /// - 2 女
        pub sex: i64,
        /// 用户等级
        pub level: u8,
        // /// ? 直播间排名
        // pub rank: i64,
        /// 用户是否被封禁
        ///
        /// - 0 未被封禁
        /// - 1 被封禁
        pub silence: i64,
        /// 大会员信息
        pub vip: Vip,
        // /// 是否已设置邮箱
        // pub email_status: i64,
        // /// 是否已设置手机
        // pub tel_status: i64,
        /// 认证信息
        pub official: Official,
        // /// ?
        // pub identification: i64,
        // /// ?
        // pub invite: Invite,
        // /// ?
        // pub is_tourist: i64,
        // /// 高亮显示(密码未设置等)
        // pub pin_prompting: i64,
        // /// ?
        // pub in_reg_audit: i64,
        // /// ?
        // pub has_face_nft: bool,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    pub struct Vip {
        /// 大会员类型
        ///
        /// - 0 无大会员
        /// - 1 月度大会员
        /// - 2 年度大会员
        pub r#type: i64,
        /// 大会员状态
        ///
        /// - 0 非大会员
        /// - 1 大会员
        /// - 2 大会员(冻结)
        pub status: i64,
        /// 大会员到期时间(毫秒时间戳)
        pub due_date: i64,
        // /// ?
        // pub vip_pay_type: i64,
        // /// ?
        // /// - 0 正常
        // /// - 1 小会员
        // pub theme_type: i64,
        // /// 大会员标识
        // pub label: Label,
        // /// 昵称颜色色标
        // pub nickname_color: String,
        // /// ?
        // pub role: i64,
        // /// 头像右下角 ICON
        // pub avatar_icon: AvatarIcon,
        // /// ?
        // pub avatar_subscript: i64,
        // /// ?
        // pub avatar_subscript_url: String,
        // /// TV 会员状态
        // pub tv_vip_status: i64,
        // /// TV 会员支付类型
        // pub tv_vip_pay_type: i64,
        // /// TV 会员到期时间(秒时间戳)
        // pub tv_due_date: i64,
    }

    // #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    // pub struct Label {
    //     pub path: String,
    //     pub text: String,
    //     pub label_theme: String,
    //     pub text_color: String,
    //     pub bg_style: i64,
    //     pub bg_color: String,
    //     pub border_color: String,
    //     pub use_img_label: bool,
    //     pub img_label_uri_hans: String,
    //     pub img_label_uri_hant: String,
    //     pub img_label_uri_hans_static: String,
    //     pub img_label_uri_hant_static: String,
    // }

    // #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    // pub struct AvatarIcon {
    //     pub icon_resource: IconResource,
    // }

    // #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    // pub struct IconResource {}

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    /// 认证信息
    pub struct Official {
        /// 认证类型
        ///
        /// - 0 未认证
        /// - 1, 2, 7 个人认证
        /// - 3, 4, 5, 6 机构认证
        pub role: i64,
        pub title: String,
        pub desc: String,
        pub r#type: i64,
    }

    // #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    // pub struct Invite {
    //     pub invite_remind: i64,
    //     pub display: bool,
    // }
}
