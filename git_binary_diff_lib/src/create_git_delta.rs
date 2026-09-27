// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use serde::{Serialize, Deserialize};

#[derive(Debug, Default, Clone, Copy, PartialOrd, PartialEq, Ord, Eq, Serialize, Deserialize)]
pub struct CommonSubsequence(pub usize, pub usize, pub usize);

/// Encodes an integer value into a standard Git variable-length integer (Varint) byte array
fn encode_varint(mut val: usize, out: &mut Vec<u8>) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val > 0 {
            byte |= 0x80; // Set MSB continuation bit
            out.push(byte);
        } else {
            out.push(byte);
            break;
        }
    }
}

/// Emits a single optimized Git Binary Copy command frame
fn emit_copy(offset: usize, mut size: usize, out: &mut Vec<u8>) {
    // Break copies into maximum single chunks of 64KB per command limit
    while size > 0 {
        let chunk_size = size.min(0x10000);
        let current_size = if chunk_size == 0x10000 { 0 } else { chunk_size };

        let mut cmd = 0x80u8;
        let mut bytes = Vec::with_capacity(7);

        // Dynamically pack Offset byte values (up to 4 bytes mapping bits 0x01..0x08)
        for i in 0..4 {
            let byte = ((offset >> (8 * i)) & 0xFF) as u8;
            if byte != 0 {
                cmd |= 0x01 << i;
                bytes.push(byte);
            }
        }

        // Dynamically pack Size byte values (up to 3 bytes mapping bits 0x10..0x40)
        for i in 0..3 {
            let byte = ((current_size >> (8 * i)) & 0xFF) as u8;
            if byte != 0 {
                cmd |= 0x10 << i;
                bytes.push(byte);
            }
        }

        out.push(cmd);
        out.extend_from_slice(&bytes);

        size -= chunk_size;
    }
}

/// Emits raw literal blocks split into compliant sub-128 byte sequences
fn emit_literal(target_data: &[u8], out: &mut Vec<u8>) {
    for chunk in target_data.chunks(127) {
        out.push(chunk.len() as u8); // Literal opcode is just its exact size
        out.extend_from_slice(chunk);
    }
}

/// Generates a binary delta patch stream out of an ordered match sequence
pub fn create_delta(
    source: &[u8],
    target: &[u8],
    matches: &[CommonSubsequence],
) -> Vec<u8> {
    let mut out = Vec::new();

    // 1. Write standard structural size header bounds
    encode_varint(source.len(), &mut out);
    encode_varint(target.len(), &mut out);

    let mut current_target_idx = 0;

    // 2. Loop over segments to weave literal replacements and copy frames
    for &CommonSubsequence(src_off, tgt_off, len) in matches {
        if len == 0 {
            continue;
        }

        // Bridge gaps in unmapped target space by emitting Literal insertion codes
        if tgt_off > current_target_idx {
            let literal_slice = &target[current_target_idx..tgt_off];
            emit_literal(literal_slice, &mut out);
        }

        // Emit copy instructions for matching blocks
        emit_copy(src_off, len, &mut out);
        current_target_idx = tgt_off + len;
    }

    // Bridge any remaining trailing bytes up to the end of the target stream
    if current_target_idx < target.len() {
        let trailing_slice = &target[current_target_idx..];
        emit_literal(trailing_slice, &mut out);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_delta::patch_delta; // Assumes your patch_delta is accessible locally

    #[test]
    fn test_delta_generation_roundtrip() {
        let source = b"abcdefghijklmnop_SHARED_DATA_xyz1234567890_TRAILING";
        let target = b"PRE_PATCH_abcdefghijklmnop_SHARED_DATA_xyz1234567890_MODIFIED";

        // Ordered match segments calculated by an LCS engine:
        // CommonSubsequence(source_offset, target_offset, segment_length)
        let matches = vec![
            CommonSubsequence(0, 10, 42), // Matches "abcdefghijklmnop_SHARED_DATA_xyz1234567890"
        ];

        // Generate the delta vector payload
        let delta = create_delta(source, target, &matches);

        // Roundtrip test: feed the payload into your newly verified patch engine!
        let patched_output = patch_delta(source, &delta)
            .expect("Generated delta could not be successfully unpacked by patch_delta!");

        assert_eq!(patched_output, target.to_vec());
    }
}
