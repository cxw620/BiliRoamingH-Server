use anyhow::{bail, Result};

use std::collections::HashMap;

use lib_utils::str_concat;

#[derive(Debug)]
pub struct AccountCommParams {
    inner: HashMap<&'static str, String>,
}

#[derive(Debug)]
pub struct PassportCommParams {
    inner: HashMap<&'static str, String>,
}

impl PassportCommParams {
    pub fn new() -> Self {
        let mut map = HashMap::with_capacity(8);
        map.insert("device", "phone".to_owned());
        Self {
            inner: HashMap::with_capacity(8),
        }
    }

    /// Set `bili_local_id`, or `fp_local`
    pub fn set_fp_local(mut self, fp_local: impl ToString) -> Self {
        self.inner.insert("bili_local_id", fp_local.to_string());
        self
    }

    /// Set `device_id`, or `fp_remote`
    pub fn set_fp_remote(mut self, device_id: impl ToString) -> Self {
        self.inner.insert("device_id", device_id.to_string());
        self
    }

    /// Set `local_id`, or `buvid_local`
    pub fn set_buvid_local(mut self, buvid: impl ToString) -> Self {
        self.inner.insert("local_id", buvid.to_string());
        self
    }

    /// Set `buvid`
    /// 
    /// Attention: `buvid` are server side stored one
    pub fn set_buvid(mut self, buvid: impl ToString) -> Self {
        self.inner.insert("buvid", buvid.to_string());
        self
    }

    /// Set `device_name`, or Build.MANUFACTURER + Build.MODEL
    ///
    /// Example: "Google" + "Pixel 2 XL"
    pub fn set_device_name(mut self, manufacturer: &str, model: &str) -> Self {
        self.inner
            .insert("device_name", str_concat!(manufacturer, model));
        self
    }

    /// Set `device_platform`, or "Android" + Build.VERSION.RELEASE + Build.MANUFACTURER + Build.MODEL
    ///
    /// Example: "Android" + "11" + "Google" + "Pixel 2 XL"
    pub fn set_device_platform(
        mut self,
        version_release: &str,
        manufacturer: &str,
        model: &str,
    ) -> Self {
        self.inner.insert(
            "device_platform",
            str_concat!("Android", version_release, manufacturer, model),
        );
        self
    }

    /// Set `from_access_key` when logined
    pub fn set_from_access_key(mut self, from_access_key: impl ToString) -> Self {
        self.inner
            .insert("from_access_key", from_access_key.to_string());
        self
    }

    #[tracing::instrument]
    pub fn build(self) -> Result<HashMap<&'static str, String>> {
        if self.inner.len() < 7 && self.inner.contains_key("from_access_key") {
            tracing::error!(
                "Build PassportCommParams error, not all required fields are set: {:?}",
                self.inner
            );
            bail!("Check if all required fields are set");
        }
        Ok(self.inner)
    }
}

impl Default for PassportCommParams {
    fn default() -> Self {
        // Self::new()
        todo!("PassportCommParams::default")
    }
}
