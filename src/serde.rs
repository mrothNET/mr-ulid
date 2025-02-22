use std::fmt;

use serde::{
    Deserialize, Serialize, Serializer,
    de::{self, Deserializer, Visitor},
};

use crate::{Ulid, ZeroableUlid, base32};

impl Serialize for ZeroableUlid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = [0; 26];
        serializer.serialize_str(base32::encode(self.to_u128(), &mut buffer))
    }
}

impl Serialize for Ulid {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut buffer = [0; 26];
        serializer.serialize_str(base32::encode(self.to_u128(), &mut buffer))
    }
}

impl<'de> Deserialize<'de> for ZeroableUlid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ZeroableVisitor;

        impl<'de> Visitor<'de> for ZeroableVisitor {
            type Value = ZeroableUlid;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid ULID string")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value.parse().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(ZeroableVisitor)
    }
}

impl<'de> Deserialize<'de> for Ulid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NonZeroVisitor;

        impl<'de> Visitor<'de> for NonZeroVisitor {
            type Value = Ulid;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a valid ULID string not all zeros chars")
            }
            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                value.parse().map_err(de::Error::custom)
            }
        }

        deserializer.deserialize_str(NonZeroVisitor)
    }
}
