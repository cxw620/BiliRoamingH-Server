use std::borrow::Cow;

#[derive(Debug)]
pub enum Signer {
    None,
}

impl Signer {
    pub fn sign<'q>(&self, _params: &mut Vec<(&'q str, Cow<'q, str>)>) -> String {
        todo!()
    }
}
