// Copyright 2024 Peter Williams <pwil3058@gmail.com> <pwil3058@bigpond.net.au>

pub mod text_diff;
pub mod unified_diff;
pub mod unified_diff_copy;

pub const TIMESTAMP_RE_STR: &str = r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(\.\d{9})? [-+]{1}\d{4}";
pub const ALT_TIMESTAMP_RE_STR: &str =
    r"[A-Z][a-z]{2} [A-Z][a-z]{2} \d{2} \d{2}:\d{2}:\d{2} \d{4} [-+]{1}\d{4}";
pub const PATH_RE_STR: &str = r###""([^"]+)"|(\S+)"###;
