use serde::{Deserialize, de::Error};

#[derive(Debug)]
#[repr(transparent)]
pub struct EnvValue<T>(T);

impl<T> EnvValue<T>
where
    T: TryFrom<String>,
    T::Error: std::error::Error,
{
    pub fn resolve(env: &str) -> anyhow::Result<Self> {
        let v = match shellexpand::env(&env)? {
            std::borrow::Cow::Borrowed(_) => T::try_from(env.to_string()),
            std::borrow::Cow::Owned(v) => T::try_from(v),
        }
        .map_err(|e| anyhow::anyhow!("Failed to resolve Env string into type {e}"))?;
        Ok(Self(v))
    }
}

impl<T> EnvValue<T> {
    pub fn into_inner(self) -> T {
        self.0
    }

    pub fn as_inner(&self) -> &T {
        &self.0
    }
}

impl<T: AsRef<str>> AsRef<str> for EnvValue<T> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T: Default> Default for EnvValue<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'de, T> Deserialize<'de> for EnvValue<T>
where
    T: TryFrom<String>,
    T::Error: std::error::Error,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let src = String::deserialize(deserializer)?;
        EnvValue::resolve(&src).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::EnvValue;
    use std::path::PathBuf;

    #[test]
    fn test_types_string() {
        let home = std::env::var("HOME").unwrap();
        let test_file = format!("{home}/file.txt");

        let _val: PathBuf = EnvValue::resolve("$HOME/file.txt").unwrap().into_inner();
        assert_eq!(_val.to_string_lossy(), test_file);

        let _val: String = EnvValue::resolve("$HOME/file.txt").unwrap().into_inner();
        assert_eq!(_val, test_file);
    }
}
