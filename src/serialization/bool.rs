use serde::de::Error;
use serde::Deserializer;

pub struct WebBool;
impl<'de> ::serde_with::DeserializeAs<'de, bool> for WebBool {
    fn deserialize_as<D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>
    {
        struct OptionWebBool;
        impl serde::de::Visitor<'_> for OptionWebBool
        {
            type Value = bool;

            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(v)
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "on" => Ok(true),
                    "off" => Ok(false),
                    v => <bool as core::str::FromStr>::from_str(v).map_err(serde::de::Error::custom),
                }
            }
        }

        deserializer.deserialize_any(OptionWebBool)
    }
}

impl ::serde_with::SerializeAs<bool> for WebBool {
    fn serialize_as<S>(source: &bool, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer
    {
        match source {
            true => serializer.serialize_str("on"),
            false => serializer.serialize_str("off"),
        }
    }
}
