pub struct Buvid {

}

// API: app.bilibili.com/x/polymer/buvid/get
pub mod x_polymer_buvid_get {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Default, Serialize, Deserialize)]
    #[serde(default)]
    pub struct RemoteBuvid {
        pub buvid: String,
        pub device_type: String,
        pub match_device: String,
    }
}
