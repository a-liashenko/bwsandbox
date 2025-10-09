use serde::{Deserialize, de::Error};
use std::env::VarError;

#[derive(Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct EnvVal<T>(T);

impl<T: From<String>> EnvVal<T> {
    #[tracing::instrument]
    pub fn resolve(val: String) -> Result<Self, shellexpand::LookupError<VarError>> {
        let value = shellexpand::env(&val)?;
        let value = match value {
            std::borrow::Cow::Owned(v) => v,
            std::borrow::Cow::Borrowed(_) => val,
        };
        Ok(Self(T::from(value)))
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'de, T: From<String>> Deserialize<'de> for EnvVal<T> {
    #[tracing::instrument(skip_all)]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let src = String::deserialize(deserializer)?;
        let value = EnvVal::resolve(src).map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basic_type() {
        let eval = std::env::var("USER").unwrap();
        let ekey = "USER";

        let src = format!("Hello {eval}!");
        let value: EnvVal<String> = EnvVal::resolve(format!("Hello ${ekey}!")).unwrap();
        assert_eq!(value.as_inner(), &src);

        let src = format!("/path/to/{eval}.file");
        let value: EnvVal<PathBuf> = EnvVal::resolve(format!("/path/to/${ekey}.file")).unwrap();
        assert_eq!(value.as_inner(), &PathBuf::from(src));
    }

    #[test]
    fn test_missing_env() {
        let ekey = crate::utils::rand_id(10);
        let value = EnvVal::<String>::resolve(format!("Hello ${ekey}_MISSING"));
        assert!(value.is_err());
    }
}
