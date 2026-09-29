// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

pub mod human_lines;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A single row of binary data formatted elegantly for human review.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct BinaryRow {
    pub offset: usize,
    pub hex: String,
    pub ascii: String,
}

pub mod human_binary {
    use super::BinaryRow;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    const ROW_SIZE: usize = 16; // Standard hex editor line size

    /// Serializes raw u8 sequences into clean, readable Hex/ASCII rows for JSON review.
    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut rows = Vec::with_capacity((bytes.len() + ROW_SIZE - 1) / ROW_SIZE);

        for (idx, chunk) in bytes.chunks(ROW_SIZE).enumerate() {
            let offset = idx * ROW_SIZE;

            // Convert bytes to space-separated hex characters: "00 1a ff"
            let hex = chunk
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<String>>()
                .join(" ");

            // Convert bytes to readable ASCII characters, using '.' for non-printable control bytes
            let ascii = chunk
                .iter()
                .map(|&b| {
                    if (32..=126).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect::<String>();

            rows.push(BinaryRow { offset, hex, ascii });
        }

        rows.serialize(serializer)
    }

    /// Deserializes human-readable hex rows seamlessly back into a strict byte stream.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Box<[u8]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let rows = Vec::<BinaryRow>::deserialize(deserializer)?;
        let mut raw_bytes = Vec::with_capacity(rows.len() * ROW_SIZE);

        for row in rows {
            // Reconstruct the bytes from the space-separated hex string
            for hex_byte in row.hex.split_whitespace() {
                let byte = u8::from_str_radix(hex_byte, 16).map_err(|e| {
                    serde::de::Error::custom(format!("Invalid hex character: {}", e))
                })?;
                raw_bytes.push(byte);
            }
        }

        Ok(raw_bytes.into_boxed_slice())
    }
}
