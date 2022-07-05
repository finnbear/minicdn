#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;

/// Like [`bytes::Bytes`] but serializes as base64.
#[derive(Clone, Default, PartialEq, Eq)]
#[repr(transparent)]
pub struct Base64Bytes(bytes::Bytes);

impl Base64Bytes {
    pub fn from_static(bytes: &'static [u8]) -> Self {
        Self(bytes::Bytes::from_static(bytes))
    }
}

impl Deref for Base64Bytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<bytes::Bytes> for Base64Bytes {
    fn from(val: bytes::Bytes) -> Self {
        Self(val)
    }
}

impl From<Vec<u8>> for Base64Bytes {
    fn from(val: Vec<u8>) -> Self {
        Self(bytes::Bytes::from(val))
    }
}

impl From<Base64Bytes> for bytes::Bytes {
    fn from(val: Base64Bytes) -> Self {
        val.0
    }
}

impl fmt::Debug for Base64Bytes {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!("b\"{}\"", base64::encode(&self.0)))
    }
}

#[cfg(feature = "serde")]
impl Serialize for Base64Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if serializer.is_human_readable() {
            let encoded = base64::encode(&self.0);
            serializer.serialize_str(&encoded)
        } else {
            self.0.serialize(serializer)
        }
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Base64Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let encoded = <&str>::deserialize(deserializer)?;
            base64::decode(encoded)
                .map_err(serde::de::Error::custom)
                .map(Into::<bytes::Bytes>::into)
                .map(Base64Bytes::from)
        } else {
            <bytes::Bytes>::deserialize(deserializer).map(Base64Bytes::from)
        }
    }
}
