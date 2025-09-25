use serde::{Deserialize, de::Error};

#[derive(Debug)]
#[repr(transparent)]
pub struct EnvValue(String);

impl EnvValue {
    pub fn resolve(env: String) -> anyhow::Result<Self> {
        let v = match shellexpand::env(&env)? {
            std::borrow::Cow::Borrowed(_) => Self(env),
            std::borrow::Cow::Owned(v) => Self(v),
        };
        Ok(v)
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for EnvValue {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl Default for EnvValue {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'de> Deserialize<'de> for EnvValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let src = String::deserialize(deserializer)?;
        EnvValue::resolve(src).map_err(D::Error::custom)
    }
}
