use super::sign::Signer;
use std::borrow::Cow;

#[derive(Debug)]
pub struct QueryBuilder<'q> {
    parameters: Vec<(&'q str, Cow<'q, str>)>,
    signer: Signer,
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
    pub fn with_signer(&mut self, signer: Signer) -> &mut Self {
        self.signer = signer;
        self
    }

    /// Set if need sort when building query string.
    pub fn with_sort(&mut self, need_sort: bool) -> &mut Self {
        self.need_sort = need_sort;
        self
    }

    /// Add a new parameter.
    pub fn add_param(&mut self, k: &'q str, v: impl Into<Cow<'q, str>>) -> &mut Self {
        self.parameters.push((k, v.into()));
        self
    }

    /// Add new parameters.
    pub fn add_params(&mut self, params: Vec<(&'q str, Cow<'q, str>)>) -> &mut Self {
        self.parameters.extend(params);
        self
    }

    /// Sort parameters **in place** by key instantly.
    pub fn sort_params(&mut self, need_sort: bool) -> &mut Self {
        if need_sort {
            sort_parameters(&mut self.parameters);
        }
        self
    }

    /// Sign the query and add sign parameter to `self.parameters`.
    ///
    /// See [`Signer`] for details.
    pub fn sign(&mut self) -> &mut Self {
        self.signer.sign(&mut self.parameters);
        self
    }

    /// Build query string.
    ///
    /// DO NOT use original builder after calling this method,
    /// since that `self.parameters` has been already consumed.
    pub fn build(&mut self) -> String {
        // Using `std::mem::take` is not good since the used builder should be dropped instead of
        // replaced with a new one, though we cannot do so behind a shared reference.
        // Compromise for chain calling?
        let params = std::mem::take(&mut self.parameters);
        encode_parameters(params, self.need_sort)
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
    mut parameters: Vec<(&'q str, Cow<'q, str>)>,
    need_sort: bool,
) -> String {
    if need_sort {
        sort_parameters(&mut parameters);
    }
    let mut parameters_str = String::with_capacity(256);
    parameters.into_iter().for_each(|(k, v)| {
        parameters_str.push_str(k);
        parameters_str.push('=');
        parameters_str.push_str(&urlencoding::encode(v.as_ref()));
        parameters_str.push('&');
    });
    parameters_str.pop();
    parameters_str
}

#[inline]
/// Sort parameters **in place** by key.
pub fn sort_parameters(parameters: &mut Vec<(&str, Cow<'_, str>)>) {
    parameters.sort_unstable_by_key(|param| param.0);
}

#[cfg(test)]
pub mod test {
    use super::*;

    #[test]
    fn test_encode_space() {
        let params = vec![("a", "b c".into()), ("c", "d".into())];
        assert_eq!(encode_parameters(params, false), "a=b%20c&c=d");
    }
}
