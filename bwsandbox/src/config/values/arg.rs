use super::{EnvVal, TempFileVal};
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, ffi::OsStr};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "type")]
pub enum ArgVal {
    Str { value: String },
    Env { value: EnvVal<String> },
    Tempfile { name: TempFileVal },
}

impl AsRef<OsStr> for ArgVal {
    fn as_ref(&self) -> &OsStr {
        match self {
            ArgVal::Str { value } => value.as_ref(),
            ArgVal::Env { value } => value.as_inner().as_ref(),
            ArgVal::Tempfile { name } => name.as_inner().as_os_str(),
        }
    }
}

impl ArgVal {
    #[tracing::instrument]
    pub fn to_str(&self) -> Cow<'_, str> {
        match self {
            ArgVal::Str { value } => Cow::Borrowed(value),
            ArgVal::Env { value } => Cow::Borrowed(value.as_inner()),
            ArgVal::Tempfile { name } => name.as_inner().to_string_lossy(),
        }
    }
}

impl Serialize for ArgVal {
    #[tracing::instrument(skip_all)]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let value = self.to_str();
        value.serialize(serializer)
    }
}
