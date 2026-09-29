// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A generic serializer that works on any collection of types that implement AsRef<str>
pub fn serialize<T, S>(lines: &[T], serializer: S) -> Result<S::Ok, S::Error>
where
    T: AsRef<str>,
    S: Serializer,
{
    let mut json_items = Vec::with_capacity(lines.len() + 1);

    for (idx, item) in lines.iter().enumerate() {
        let line = item.as_ref();
        if line.ends_with('\n') {
            let clean = line.trim_end_matches(|c| c == '\n' || c == '\r');
            json_items.push(clean.to_string());
        } else {
            json_items.push(line.to_string());
            if idx == lines.len() - 1 {
                json_items.push(String::from(r"\ No newline at end of file"));
            }
        }
    }

    json_items.serialize(serializer)
}

/// Deserializes straight back into concrete String blocks
pub fn deserialize<'de, D>(deserializer: D) -> Result<Box<[String]>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut json_items = Vec::<String>::deserialize(deserializer)?;
    if json_items.is_empty() {
        return Ok(vec![].into_boxed_slice());
    }

    let has_no_newline_eof = json_items
        .last()
        .map_or(false, |s| s == r"\ No newline at end of file");
    if has_no_newline_eof {
        json_items.pop();
    }

    let len = json_items.len();
    let mut runtime_lines = Vec::with_capacity(len);

    for (idx, mut line) in json_items.into_iter().enumerate() {
        if idx < len - 1 || !has_no_newline_eof {
            line.push('\n');
        }
        runtime_lines.push(line);
    }

    Ok(runtime_lines.into_boxed_slice())
}
