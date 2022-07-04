use ref_cast::RefCast;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::{Borrow, Cow};
use std::ops::{Deref, DerefMut};

#[derive(Debug, PartialEq, Eq, RefCast)]
#[repr(transparent)]
pub struct Bytes([u8]);

impl<'a> From<&'a Bytes> for &'a [u8] {
    fn from(val: &'a Bytes) -> Self {
        // SAFETY: Same memory layout, as checked by [`RefCast`] in the other direction.
        unsafe { &*(val as *const Bytes as *const [u8]) }
    }
}

impl<'a> From<&'a [u8]> for &'a Bytes {
    fn from(val: &'a [u8]) -> Self {
        RefCast::ref_cast(val)
    }
}

impl Deref for Bytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Bytes {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl ToOwned for Bytes {
    type Owned = ByteBuf;

    fn to_owned(&self) -> Self::Owned {
        self.0.to_owned().into()
    }
}

impl Serialize for Bytes {
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

impl<'de> Deserialize<'de> for &Bytes {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        panic!("cannot deserialize borrowed bytes");
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ByteBuf(Vec<u8>);

impl Deref for ByteBuf {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ByteBuf {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<u8>> for ByteBuf {
    fn from(val: Vec<u8>) -> Self {
        ByteBuf(val)
    }
}

impl From<ByteBuf> for Vec<u8> {
    fn from(val: ByteBuf) -> Self {
        val.0
    }
}

impl Borrow<Bytes> for ByteBuf {
    fn borrow(&self) -> &Bytes {
        <Vec<u8> as Borrow<[u8]>>::borrow(&self.0).into()
    }
}

impl Serialize for ByteBuf {
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

impl<'de> Deserialize<'de> for ByteBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        if deserializer.is_human_readable() {
            let encoded = <&str>::deserialize(deserializer)?;
            base64::decode(encoded)
                .map_err(serde::de::Error::custom)
                .map(ByteBuf::from)
        } else {
            <Vec<u8>>::deserialize(deserializer).map(ByteBuf::from)
        }
    }
}

pub fn convert_serde_base64_cow(cow: Cow<'_, Bytes>) -> Cow<'_, [u8]> {
    match cow {
        Cow::Borrowed(bytes) => Cow::Borrowed(bytes.into()),
        Cow::Owned(byte_buf) => Cow::Owned(byte_buf.into()),
    }
}
