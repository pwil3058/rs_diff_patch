// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod human_lines;

pub mod human_binary {
    use base64::{Engine, prelude::BASE64_STANDARD};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Encodes raw bytes into a single, ultra-compact Base64 string for JSON storage
    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoded = BASE64_STANDARD.encode(bytes);
        encoded.serialize(serializer)
    }

    /// Decodes the compact Base64 string back into exact raw binary bytes
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Box<[u8]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded_string = String::deserialize(deserializer)?;
        let decoded_bytes = BASE64_STANDARD
            .decode(encoded_string)
            .map_err(|e| serde::de::Error::custom(format!("Invalid Base64 token payload: {e}")))?;

        Ok(decoded_bytes.into_boxed_slice())
    }
}
