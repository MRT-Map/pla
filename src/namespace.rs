use std::{
    fmt::{Debug, Display, Formatter},
    ops::Deref,
    path::Path,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use smol_str::SmolStr;

use crate::{Error, Result};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct Namespace(SmolStr);

impl Namespace {
    pub fn str_is_valid<S: AsRef<str> + ?Sized>(s: &S) -> bool {
        !s.as_ref().is_empty()
            && s.as_ref()
                .chars()
                .all(|a| a.is_ascii_alphanumeric() || a == '_')
    }
    pub fn new<S: AsRef<str> + ?Sized>(s: &S) -> Result<Self> {
        let s = s.as_ref();
        if !Self::str_is_valid(&s) {
            return Err(Error::InvalidNamespace(s.to_owned()));
        }
        Ok(Self(s.into()))
    }
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Namespace {
    fn default() -> Self {
        Self("default".into())
    }
}

impl Display for Namespace {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl Debug for Namespace {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Deref for Namespace {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AsRef<Path> for Namespace {
    fn as_ref(&self) -> &Path {
        self.0.as_str().as_ref()
    }
}
impl AsRef<str> for Namespace {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

#[cfg(feature = "serde")]
impl Serialize for Namespace {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Namespace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}
