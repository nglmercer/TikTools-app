//! Backward-compatible durable consent identities.
//!
//! Original u64 epochs remain JSON numbers so an older frozen journal can be
//! read without rewriting it. New 128-bit epochs are canonical decimal text:
//! JSON runtimes cannot reliably represent those values as numbers.
use serde::{
    de::{self, Visitor},
    Deserializer, Serializer,
};
use std::fmt;

pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if let Ok(legacy) = u64::try_from(*value) {
        serializer.serialize_u64(legacy)
    } else {
        serializer.serialize_str(&value.to_string())
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    struct Generation;
    impl Visitor<'_> for Generation {
        type Value = u128;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a legacy unsigned generation or canonical decimal consent identity")
        }
        fn visit_u64<E: de::Error>(self, value: u64) -> Result<u128, E> {
            Ok(value.into())
        }
        fn visit_str<E: de::Error>(self, value: &str) -> Result<u128, E> {
            let parsed = value
                .parse::<u128>()
                .map_err(|_| E::custom("invalid consent identity"))?;
            if parsed <= u64::MAX.into() || parsed.to_string() != value {
                return Err(E::custom("noncanonical consent identity"));
            }
            Ok(parsed)
        }
    }
    deserializer.deserialize_any(Generation)
}

#[cfg(test)]
mod tests {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Example {
        #[serde(with = "super")]
        generation: u128,
    }

    #[test]
    fn legacy_numbers_remain_byte_compatible_and_new_values_use_decimal_strings() {
        let legacy = r#"{"generation":42}"#;
        let parsed: Example = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed.generation, 42);
        assert_eq!(serde_json::to_string(&parsed).unwrap(), legacy);
        let large = Example {
            generation: u128::MAX,
        };
        let encoded = serde_json::to_string(&large).unwrap();
        assert_eq!(encoded, format!(r#"{{"generation":"{}"}}"#, u128::MAX));
        assert_eq!(serde_json::from_str::<Example>(&encoded).unwrap(), large);
        for invalid in [
            r#"{"generation":"42"}"#,
            r#"{"generation":"00018446744073709551616"}"#,
            r#"{"generation":-1}"#,
        ] {
            assert!(serde_json::from_str::<Example>(invalid).is_err());
        }
    }
}
