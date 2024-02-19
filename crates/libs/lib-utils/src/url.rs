use anyhow::Result;
use std::borrow::Cow;

use super::error::ServerError;
use super::sign::{Signer, Wbi};
use crate::{now, str_concat};

#[derive(Debug)]
pub struct QueryBuilder<'q> {
    parameters: Vec<(&'q str, Cow<'q, str>)>,
    signer: Signer<'q>,
    need_sort: bool,
}

impl Default for QueryBuilder<'_> {
    fn default() -> Self {
        Self {
            parameters: Vec::with_capacity(64),
            signer: Signer::None,
            need_sort: false,
        }
    }
}

impl<'q> QueryBuilder<'q> {
    /// Init a new QueryBuilder.
    pub fn new(parameters: Vec<(&'q str, Cow<'q, str>)>) -> Self {
        Self {
            parameters,
            ..Default::default()
        }
    }

    /// Set signer.
    pub fn with_signer(mut self, signer: Signer<'q>) -> Self {
        self.signer = signer;
        self
    }

    /// Set if need sort when building query string.
    ///
    /// May not take effect when need sign.
    pub fn with_sort(mut self, need_sort: bool) -> Self {
        self.need_sort = need_sort;
        self
    }

    /// Add a new parameter.
    pub fn add_param(mut self, k: &'q str, v: impl Into<Cow<'q, str>>) -> Self {
        self.parameters.push((k, v.into()));
        self
    }

    /// Add new parameters.
    pub fn add_params(mut self, params: Vec<(&'q str, Cow<'q, str>)>) -> Self {
        self.parameters.extend(params);
        self
    }

    /// Build query string.
    ///
    /// May failed due to sign error.
    pub fn build(mut self) -> Result<String> {
        match self.signer {
            Signer::None => {
                let mut params = std::mem::take(&mut self.parameters);
                Ok(encode_parameters(&mut params, self.need_sort))
            }
            Signer::Wbi { img_key, sub_key } => {
                let mixin_key = Wbi::gen_mixin_key(img_key, sub_key)?;

                let wts = if cfg!(test) {
                    "1703513649".to_owned()
                } else {
                    now!().as_secs().to_string()
                };

                let wts_param = str_concat!("wts=", &wts);
                self.parameters.push(("wts", wts.into()));

                let unsigned_query = encode_parameters(&mut self.parameters, true);

                let w_rid = Wbi::gen_w_rid(&unsigned_query, &mixin_key);

                // `wts`, `w_rid` should add to the end of unsigned query.
                // With this we needn't encode parameters twice.
                let signed_query = {
                    // `wts_param` will and will only appear once in unsigned_query
                    let (mut start, part) =
                        unsigned_query.match_indices(&wts_param).next().unwrap();
                    let mut part_len = part.len();
                    // `start` > 0 then not the first, should also remove `&` before `wts`
                    if start != 0 {
                        start -= 1;
                        part_len += 1;
                    }
                    // SAFE: Will not out of bound
                    str_concat!(
                        unsafe { unsigned_query.get_unchecked(0..start) },
                        unsafe {
                            unsigned_query.get_unchecked((start + part_len)..unsigned_query.len())
                        },
                        "&w_rid=",
                        &w_rid,
                        "&",
                        &wts_param
                    )
                };

                Ok(signed_query)
            }
        }
    }
}

/// Encode query string from given parameters.
///
/// # WARNING
///
/// For performance:
/// - **KEY will not be url encoded**, for all query key is known and without special chars.
/// - When `need_sort`, the given vec will be sorted in place. **Do clone a new one in advance**
///   if you need the original one.
#[inline]
pub fn encode_parameters<'q>(
    parameters: &mut Vec<(&'q str, Cow<'q, str>)>,
    need_sort: bool,
) -> String {
    if need_sort {
        sort_parameters(parameters);
    }
    let mut parameters_str = String::with_capacity(256);
    parameters.iter().for_each(|(k, v)| {
        parameters_str.push_str(k);
        parameters_str.push('=');
        parameters_str.push_str(&urlencoding::encode(v.as_ref()));
        parameters_str.push('&');
    });
    parameters_str.pop();
    parameters_str
}

#[inline(always)]
/// Sort parameters **in place** by key.
pub fn sort_parameters(parameters: &mut Vec<(&str, Cow<'_, str>)>) {
    parameters.sort_unstable_by_key(|param| param.0);
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_encode_space() {
        let mut params = vec![("a", "b c".into()), ("c", "d".into())];
        assert_eq!(encode_parameters(&mut params, false), "a=b%20c&c=d");
    }

    #[test]
    fn test_wbi_sign() {
        const IMG_KEY: &'static str = "7cd084941338484aae1ad9425b84077c";
        const SUB_KEY: &'static str = "4932caff0ff746eab6f01bf08b70ac45";

        let parameters = vec![
            ("mid", "11997177".into()),
            ("token", "".into()),
            ("platform", "web".into()),
            ("web_location", "1550101".into()),
        ];

        let signed_url = QueryBuilder::new(parameters)
            .with_signer(Signer::Wbi {
                img_key: IMG_KEY,
                sub_key: SUB_KEY,
            })
            .build()
            .unwrap();

        assert_eq!(signed_url, "mid=11997177&platform=web&token=&web_location=1550101&w_rid=7d4428b3f2f9ee2811e116ec6fd41a4f&wts=1703513649");
    }
}

/// For faster url query parsing usage onlly
pub type RawQueryMap<'m> = std::collections::HashMap<Cow<'m, str>, Cow<'m, str>>;

/// High performance struct for parsing query
#[derive(Debug)]
pub struct QueryMap<'m> {
    inner: RawQueryMap<'m>,
}

impl<'m> From<RawQueryMap<'m>> for QueryMap<'m> {
    fn from(value: RawQueryMap<'m>) -> Self {
        Self { inner: value }
    }
}

impl<'m> QueryMap<'m> {
    #[inline]
    pub fn get(&'m self, k: &str) -> Option<&'m str> {
        self.inner.get(k).map(|v| v.as_ref())
    }

    pub fn inner(&'m self) -> &'m RawQueryMap<'m> {
        &self.inner
    }

    pub fn inner_mut(&'m mut self) -> &'m mut RawQueryMap<'m> {
        &mut self.inner
    }

    pub fn into_inner(self) -> RawQueryMap<'m> {
        self.inner
    }

    /// Convert to `Vec<(Cow<'m, str>, Cow<'m, str>)>`. **EXPENSIVE**
    pub fn into_vec(self) -> Vec<(Cow<'m, str>, Cow<'m, str>)> {
        self.inner.into_iter().collect()
    }

    /// Convert to `Vec<(String, String)>`. **VERY EXPENSIVE**
    pub fn into_vec_owned(self) -> Vec<(String, String)> {
        self.inner
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[inline]
    #[tracing::instrument(level = "debug", name = "QueryMap.try_from_req", err)]
    /// Try to parse query from axum request.
    ///
    /// Attention: FatalReqParamInvalid if query is missing.
    pub fn try_from_req(req: &'m axum::extract::Request) -> Result<Self> {
        let query = req.uri().query().ok_or(ServerError::FatalReqParamInvalid)?;
        Self::try_from_str(query)
    }

    #[inline]
    #[tracing::instrument(level = "debug", name = "QueryMap.try_from_uri", err)]
    /// Try to parse query from http::Uri.
    ///
    /// Attention: FatalReqParamInvalid if query is missing.
    pub fn try_from_uri(uri: &'m http::Uri) -> Result<Self> {
        let query = uri.query().ok_or(ServerError::FatalReqParamInvalid)?;
        Self::try_from_str(query)
    }

    #[inline]
    pub fn try_from_str(query: &'m str) -> Result<Self> {
        let query_map: QueryMap<'m> = fluent_uri::enc::EStr::new(query)
            .split('&')
            .filter_map(|pair| pair.split_once('='))
            .map(|(k, v)| (k.decode(), v.decode()))
            .filter_map(|(k, v)| k.into_string().ok().zip(v.into_string().ok())) // ! Will ignore param with invalid UTF-8 bytes
            .collect::<std::collections::HashMap<_, _>>() // TODO Use AHashMap instead
            .into();
        Ok(query_map)
    }
}
