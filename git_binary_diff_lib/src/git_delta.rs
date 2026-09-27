// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::num::NonZeroU32;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DeltaError {
    #[error("Patch error: {0}")]
    PatchError(String),
    #[error("Empty buffer")]
    EmptyBuffer,
    #[error("Empty source  buffer")]
    EmptySourceBuffer,
    #[error("Empty target buffer")]
    EmptyTargetBuffer,
    #[error("Invalid data")]
    InvalidDelta,
    #[error("Invalid source size")]
    InvalidSourceSize,
}

const RABIN_SHIFT: usize = 23;
const RABIN_WINDOW: usize = 16;
const HASH_LIMIT: usize = 64;

#[rustfmt::skip]
const TANGO: [u32; 256] = [
    0x0000_0000, 0xab59_b4d1, 0x56b3_69a2, 0xfdea_dd73, 0x063f_6795, 0xad66_d344, 0x508c_0e37, 0xfbd5_bae6,
    0x0c7e_cf2a, 0xa727_7bfb, 0x5acd_a688, 0xf194_1259, 0x0a41_a8bf, 0xa118_1c6e, 0x5cf2_c11d, 0xf7ab_75cc,
    0x18fd_9e54, 0xb3a4_2a85, 0x4e4e_f7f6, 0xe517_4327, 0x1ec2_f9c1, 0xb59b_4d10, 0x4871_9063, 0xe328_24b2,
    0x1483_517e, 0xbfda_e5af, 0x4230_38dc, 0xe969_8c0d, 0x12bc_36eb, 0xb9e5_823a, 0x440f_5f49, 0xef56_eb98,
    0x31fb_3ca8, 0x9aa2_8879, 0x6748_550a, 0xcc11_e1db, 0x37c4_5b3d, 0x9c9d_efec, 0x6177_329f, 0xca2e_864e,
    0x3d85_f382, 0x96dc_4753, 0x6b36_9a20, 0xc06f_2ef1, 0x3bba_9417, 0x90e3_20c6, 0x6d09_fdb5, 0xc650_4964,
    0x2906_a2fc, 0x825f_162d, 0x7fb5_cb5e, 0xd4ec_7f8f, 0x2f39_c569, 0x8460_71b8, 0x798a_accb, 0xd2d3_181a,
    0x2578_6dd6, 0x8e21_d907, 0x73cb_0474, 0xd892_b0a5, 0x2347_0a43, 0x881e_be92, 0x75f4_63e1, 0xdead_d730,
    0x63f6_7950, 0xc8af_cd81, 0x3545_10f2, 0x9e1c_a423, 0x65c9_1ec5, 0xce90_aa14, 0x337a_7767, 0x9823_c3b6,
    0x6f88_b67a, 0xc4d1_02ab, 0x393b_dfd8, 0x9262_6b09, 0x69b7_d1ef, 0xc2ee_653e, 0x3f04_b84d, 0x945d_0c9c,
    0x7b0b_e704, 0xd052_53d5, 0x2db8_8ea6, 0x86e1_3a77, 0x7d34_8091, 0xd66d_3440, 0x2b87_e933, 0x80de_5de2,
    0x7775_282e, 0xdc2c_9cff, 0x21c6_418c, 0x8a9f_f55d, 0x714a_4fbb, 0xda13_fb6a, 0x27f9_2619, 0x8ca0_92c8,
    0x520d_45f8, 0xf954_f129, 0x04be_2c5a, 0xafe7_988b, 0x5432_226d, 0xff6b_96bc, 0x0281_4bcf, 0xa9d8_ff1e,
    0x5e73_8ad2, 0xf52a_3e03, 0x08c0_e370, 0xa399_57a1, 0x584c_ed47, 0xf315_5996, 0x0eff_84e5, 0xa5a6_3034,
    0x4af0_dbac, 0xe1a9_6f7d, 0x1c43_b20e, 0xb71a_06df, 0x4ccf_bc39, 0xe796_08e8, 0x1a7c_d59b, 0xb125_614a,
    0x468e_1486, 0xedd7_a057, 0x103d_7d24, 0xbb64_c9f5, 0x40b1_7313, 0xebe8_c7c2, 0x1602_1ab1, 0xbd5b_ae60,
    0x6cb5_4671, 0xc7ec_f2a0, 0x3a06_2fd3, 0x915f_9b02, 0x6a8a_21e4, 0xc1d3_9535, 0x3c39_4846, 0x9760_fc97,
    0x60cb_895b, 0xcb92_3d8a, 0x3678_e0f9, 0x9d21_5428, 0x66f4_eece, 0xcdad_5a1f, 0x3047_876c, 0x9b1e_33bd,
    0x7448_d825, 0xdf11_6cf4, 0x22fb_b187, 0x89a2_0556, 0x7277_bfb0, 0xd92e_0b61, 0x24c4_d612, 0x8f9d_62c3,
    0x7836_170f, 0xd36f_a3de, 0x2e85_7ead, 0x85dc_ca7c, 0x7e09_709a, 0xd550_c44b, 0x28ba_1938, 0x83e3_ade9,
    0x5d4e_7ad9, 0xf617_ce08, 0x0bfd_137b, 0xa0a4_a7aa, 0x5b71_1d4c, 0xf028_a99d, 0x0dc2_74ee, 0xa69b_c03f,
    0x5130_b5f3, 0xfa69_0122, 0x0783_dc51, 0xacda_6880, 0x570f_d266, 0xfc56_66b7, 0x01bc_bbc4, 0xaae5_0f15,
    0x45b3_e48d, 0xeeea_505c, 0x1300_8d2f, 0xb859_39fe, 0x438c_8318, 0xe8d5_37c9, 0x153f_eaba, 0xbe66_5e6b,
    0x49cd_2ba7, 0xe294_9f76, 0x1f7e_4205, 0xb427_f6d4, 0x4ff2_4c32, 0xe4ab_f8e3, 0x1941_2590, 0xb218_9141,
    0x0f43_3f21, 0xa41a_8bf0, 0x59f0_5683, 0xf2a9_e252, 0x097c_58b4, 0xa225_ec65, 0x5fcf_3116, 0xf496_85c7,
    0x033d_f00b, 0xa864_44da, 0x558e_99a9, 0xfed7_2d78, 0x0502_979e, 0xae5b_234f, 0x53b1_fe3c, 0xf8e8_4aed,
    0x17be_a175, 0xbce7_15a4, 0x410d_c8d7, 0xea54_7c06, 0x1181_c6e0, 0xbad8_7231, 0x4732_af42, 0xec6b_1b93,
    0x1bc0_6e5f, 0xb099_da8e, 0x4d73_07fd, 0xe62a_b32c, 0x1dff_09ca, 0xb6a6_bd1b, 0x4b4c_6068, 0xe015_d4b9,
    0x3eb8_0389, 0x95e1_b758, 0x680b_6a2b, 0xc352_defa, 0x3887_641c, 0x93de_d0cd, 0x6e34_0dbe, 0xc56d_b96f,
    0x32c6_cca3, 0x999f_7872, 0x6475_a501, 0xcf2c_11d0, 0x34f9_ab36, 0x9fa0_1fe7, 0x624a_c294, 0xc913_7645,
    0x2645_9ddd, 0x8d1c_290c, 0x70f6_f47f, 0xdbaf_40ae, 0x207a_fa48, 0x8b23_4e99, 0x76c9_93ea, 0xdd90_273b,
    0x2a3b_52f7, 0x8162_e626, 0x7c88_3b55, 0xd7d1_8f84, 0x2c04_3562, 0x875d_81b3, 0x7ab7_5cc0, 0xd1ee_e811,
];

/// Represents an active chain iterator for tracking hash collisions
pub struct DeltaIndexEntries<'a> {
    index: &'a DeltaIndex<'a>,
    next_idx: Option<NonZeroU32>,
    target_val: u32,
    count: usize,
}

impl<'a> Iterator for DeltaIndexEntries<'a> {
    type Item = usize; // Returns the file offset position

    fn next(&mut self) -> Option<Self::Item> {
        // Enforce the uniform bucket exploration limit (HASH_LIMIT)
        if self.count >= HASH_LIMIT {
            return None;
        }

        while let Some(curr) = self.next_idx {
            let idx = curr.get() as usize - 1;
            let entry_val = self.index.entry_vals[idx];
            let next_node = self.index.entry_next[idx];

            // Advance state machine pointer immediately
            self.next_idx = next_node;

            if entry_val == self.target_val {
                self.count += 1;
                return Some(self.index.entry_offsets[idx]);
            }
        }
        None
    }
}

pub struct DeltaIndex<'a> {
    _data: &'a [u8],
    hash_mask: usize,
    /// Head pointers into entry arrays. 0 denotes an unassigned/empty bucket.
    hash_heads: Vec<Option<NonZeroU32>>,
    /// Parallel flat arrays tracking raw metadata safely without pointer graphs
    entry_offsets: Vec<usize>,
    entry_vals: Vec<u32>,
    entry_next: Vec<Option<NonZeroU32>>,
}

impl<'a> DeltaIndex<'a> {
    pub fn new(data: &[u8]) -> DeltaIndex<'_> {
        // Determine index hash size.  Note that indexing skips the
        // first byte to allow for optimizing the Rabin's polynomial
        // initialization in create_delta().
        // Current delta format can't encode offsets into
        // reference buffer with more than 32 bits.
        let num_entries = (data.len().min(0xFFFF_FFFF) - 1) / RABIN_WINDOW;
        let h_size = (num_entries / 4).next_power_of_two().max(16);
        let hash_mask = h_size - 1;

        let mut hash_heads: Vec<Option<NonZeroU32>> = vec![None; h_size];

        // Preallocate flat tracking spaces based on max entry bounds
        let mut entry_offsets = Vec::with_capacity(num_entries * RABIN_WINDOW);
        let mut entry_vals = Vec::with_capacity(num_entries * RABIN_WINDOW);
        let mut entry_next = Vec::with_capacity(num_entries * RABIN_WINDOW);

        // Populate the index from bottom up
        for offset in (0..num_entries * RABIN_WINDOW - RABIN_WINDOW).rev() {
            let mut val: u32 = 0;
            for datum in &data[offset + 1..=offset + RABIN_WINDOW] {
                val = (((val << 8) & 0xFFFF_FFFF) | *datum as u32)
                    ^ TANGO[(val >> RABIN_SHIFT) as usize];

                let hash_index = (val as usize) & hash_mask;
                let head = hash_heads[hash_index];

                // Optimisation check: Collapse adjacent consecutive identical data sequences
                if let Some(first_node) = head {
                    let idx = first_node.get() as usize - 1;
                    if entry_vals[idx] == val {
                        // Mutate block offset context linearly without needing inner Mutex/Cell wraps
                        entry_offsets[idx] = offset + RABIN_WINDOW;
                        continue;
                    }
                }

                // Register a brand new index tracker node within our flat workspace
                let next_node_idx = entry_offsets.len();
                entry_offsets.push(offset + RABIN_WINDOW);
                entry_vals.push(val);
                // The new node points downstream to the old bucket head
                entry_next.push(head);

                // Update hash table routing map to point here
                hash_heads[hash_index] = NonZeroU32::new((next_node_idx + 1) as u32);
            }
        }

        DeltaIndex {
            _data: data,
            hash_mask,
            hash_heads,
            entry_offsets,
            entry_vals,
            entry_next,
        }
    }

    /// Fetches a clean data cursor iterator for target value patterns
    pub fn iter_entries(&self, val: usize) -> DeltaIndexEntries<'_> {
        let target_val = val as u32;
        let hash_index = (val) & self.hash_mask;
        DeltaIndexEntries {
            index: self,
            next_idx: self.hash_heads[hash_index],
            target_val,
            count: 0,
        }
    }
}

const DELTA_SIZE_MIN: usize = 4;

pub fn get_delta_hdr_size(delta: &[u8]) -> Result<(usize, usize), DeltaError> {
    let mut size = 0;
    let mut lshft = 0;
    let mut index = 0;
    loop {
        if index >= delta.len() {
            return Err(DeltaError::InvalidDelta);
        }
        let cmd = delta[index] as usize;
        index += 1;
        size |= (cmd & 0x7F) << lshft;
        lshft += 7;
        if cmd & 0x80 == 0 {
            break;
        }
    }
    Ok((size, index))
}

pub fn patch_delta(source: &[u8], delta: &[u8]) -> Result<Vec<u8>, DeltaError> {
    if delta.len() < DELTA_SIZE_MIN {
        return Err(DeltaError::InvalidDelta);
    }
    let mut index = 0;

    // make sure the source size matches what we expect
    let (size, bytes_used) = get_delta_hdr_size(&delta[index..])?;
    index += bytes_used;
    if size != source.len() {
        return Err(DeltaError::InvalidSourceSize);
    }

    // now the expected result size
    let (expected_size, bytes_used) = get_delta_hdr_size(&delta[index..])?;
    index += bytes_used;

    let mut output: Vec<u8> = Vec::with_capacity(expected_size);

    while index < delta.len() {
        let cmd = delta[index];
        index += 1;

        if cmd & 0x80 != 0 {
            let mut cp_offset: usize = 0;
            let mut cp_size: usize = 0;

            // Parse Copy Offset (Bits 0x01, 0x02, 0x04, 0x08)
            for i in 0..4 {
                if cmd & (0x01u8 << i) != 0 {
                    if index >= delta.len() {
                        return Err(DeltaError::InvalidDelta);
                    }
                    cp_offset |= (delta[index] as usize) << (8 * i);
                    index += 1;
                }
            }

            for i in 0..3 {
                if cmd & (0x10u8 << i) != 0 {
                    if index >= delta.len() {
                        return Err(DeltaError::InvalidDelta);
                    }
                    cp_size |= (delta[index] as usize) << (8 * i);
                    index += 1;
                }
            }

            if cp_size == 0 {
                cp_size = 0x10000;
            }

            // Boundary Protection checking source and destination constraints
            if cp_offset + cp_size > source.len() || output.len() + cp_size > expected_size {
                return Err(DeltaError::PatchError(
                    "Copy parameters out of bounds".to_string(),
                ));
            }

            output.extend_from_slice(&source[cp_offset..cp_offset + cp_size]);
        } else if cmd != 0 {
            // Literal Action: 'cmd' represents the exact byte length to emit directly
            let literal_len = cmd as usize;

            if index + literal_len > delta.len() || output.len() + literal_len > expected_size {
                return Err(DeltaError::PatchError(
                    "Literal insertion out of bounds".to_string(),
                ));
            }

            output.extend_from_slice(&delta[index..index + literal_len]);
            index += literal_len;
        } else {
            return Err(DeltaError::PatchError(
                "unexpected delta opcode 0".to_string(),
            ));
        }
    }
    if index != delta.len() || expected_size != output.len() {
        let msg = format!(
            "Delta structural length mismatch {0}:{1}:{2}",
            index,
            expected_size,
            output.len()
        );
        return Err(DeltaError::PatchError(msg));
    }

    Ok(output)
}
