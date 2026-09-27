// Copyright (c) 2026 Peter Williams <pwil3058@bigpond.net.au> <pwil3058@gmail.com>.

use std::cell::Cell;
use std::rc::Rc;

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

#[rustfmt::skip]
const _UNIFORM: [u32; 256] = [
    0x0000_0000, 0x7eb5_200d, 0x5633_f4cb, 0x2886_d4c6, 0x073e_5d47, 0x798b_7d4a, 0x510d_a98c, 0x2fb8_8981,
    0x0e7c_ba8e, 0x70c9_9a83, 0x584f_4e45, 0x26fa_6e48, 0x0942_e7c9, 0x77f7_c7c4, 0x5f71_1302, 0x21c4_330f,
    0x1cf9_751c, 0x624c_5511, 0x4aca_81d7, 0x347f_a1da, 0x1bc7_285b, 0x6572_0856, 0x4df4_dc90, 0x3341_fc9d,
    0x1285_cf92, 0x6c30_ef9f, 0x44b6_3b59, 0x3a03_1b54, 0x15bb_92d5, 0x6b0e_b2d8, 0x4388_661e, 0x3d3d_4613,
    0x39f2_ea38, 0x4747_ca35, 0x6fc1_1ef3, 0x1174_3efe, 0x3ecc_b77f, 0x4079_9772, 0x68ff_43b4, 0x164a_63b9,
    0x378e_50b6, 0x493b_70bb, 0x61bd_a47d, 0x1f08_8470, 0x30b0_0df1, 0x4e05_2dfc, 0x6683_f93a, 0x1836_d937,
    0x250b_9f24, 0x5bbe_bf29, 0x7338_6bef, 0x0d8d_4be2, 0x2235_c263, 0x5c80_e26e, 0x7406_36a8, 0x0ab3_16a5,
    0x2b77_25aa, 0x55c2_05a7, 0x7d44_d161, 0x03f1_f16c, 0x2c49_78ed, 0x52fc_58e0, 0x7a7a_8c26, 0x04cf_ac2b,
    0x73e5_d470, 0x0d50_f47d, 0x25d6_20bb, 0x5b63_00b6, 0x74db_8937, 0x0a6e_a93a, 0x22e8_7dfc, 0x5c5d_5df1,
    0x7d99_6efe, 0x032c_4ef3, 0x2baa_9a35, 0x551f_ba38, 0x7aa7_33b9, 0x0412_13b4, 0x2c94_c772, 0x5221_e77f,
    0x6f1c_a16c, 0x11a9_8161, 0x392f_55a7, 0x479a_75aa, 0x6822_fc2b, 0x1697_dc26, 0x3e11_08e0, 0x40a4_28ed,
    0x6160_1be2, 0x1fd5_3bef, 0x3753_ef29, 0x49e6_cf24, 0x665e_46a5, 0x18eb_66a8, 0x306d_b26e, 0x4ed8_9263,
    0x4a17_3e48, 0x34a2_1e45, 0x1c24_ca83, 0x6291_ea8e, 0x4d29_630f, 0x339c_4302, 0x1b1a_97c4, 0x65af_b7c9,
    0x446b_84c6, 0x3ade_a4cb, 0x1258_700d, 0x6ced_5000, 0x4355_d981, 0x3de0_f98c, 0x1566_2d4a, 0x6bd3_0d47,
    0x56ee_4b54, 0x285b_6b59, 0x00dd_bf9f, 0x7e68_9f92, 0x51d0_1613, 0x2f65_361e, 0x07e3_e2d8, 0x7956_c2d5,
    0x5892_f1da, 0x2627_d1d7, 0x0ea1_0511, 0x7014_251c, 0x5fac_ac9d, 0x2119_8c90, 0x099f_5856, 0x772a_785b,
    0x4c92_1c31, 0x3227_3c3c, 0x1aa1_e8fa, 0x6414_c8f7, 0x4bac_4176, 0x3519_617b, 0x1d9f_b5bd, 0x632a_95b0,
    0x42ee_a6bf, 0x3c5b_86b2, 0x14dd_5274, 0x6a68_7279, 0x45d0_fbf8, 0x3b65_dbf5, 0x13e3_0f33, 0x6d56_2f3e,
    0x506b_692d, 0x2ede_4920, 0x0658_9de6, 0x78ed_bdeb, 0x5755_346a, 0x29e0_1467, 0x0166_c0a1, 0x7fd3_e0ac,
    0x5e17_d3a3, 0x20a2_f3ae, 0x0824_2768, 0x7691_0765, 0x5929_8ee4, 0x279c_aee9, 0x0f1a_7a2f, 0x71af_5a22,
    0x7560_f609, 0x0bd5_d604, 0x2353_02c2, 0x5de6_22cf, 0x725e_ab4e, 0x0ceb_8b43, 0x246d_5f85, 0x5ad8_7f88,
    0x7b1c_4c87, 0x05a9_6c8a, 0x2d2f_b84c, 0x539a_9841, 0x7c22_11c0, 0x0297_31cd, 0x2a11_e50b, 0x54a4_c506,
    0x6999_8315, 0x172c_a318, 0x3faa_77de, 0x411f_57d3, 0x6ea7_de52, 0x1012_fe5f, 0x3894_2a99, 0x4621_0a94,
    0x67e5_399b, 0x1950_1996, 0x31d6_cd50, 0x4f63_ed5d, 0x60db_64dc, 0x1e6e_44d1, 0x36e8_9017, 0x485d_b01a,
    0x3f77_c841, 0x41c2_e84c, 0x6944_3c8a, 0x17f1_1c87, 0x3849_9506, 0x46fc_b50b, 0x6e7a_61cd, 0x10cf_41c0,
    0x310b_72cf, 0x4fbe_52c2, 0x6738_8604, 0x198d_a609, 0x3635_2f88, 0x4880_0f85, 0x6006_db43, 0x1eb3_fb4e,
    0x238e_bd5d, 0x5d3b_9d50, 0x75bd_4996, 0x0b08_699b, 0x24b0_e01a, 0x5a05_c017, 0x7283_14d1, 0x0c36_34dc,
    0x2df2_07d3, 0x5347_27de, 0x7bc1_f318, 0x0574_d315, 0x2acc_5a94, 0x5479_7a99, 0x7cff_ae5f, 0x024a_8e52,
    0x0685_2279, 0x7830_0274, 0x50b6_d6b2, 0x2e03_f6bf, 0x01bb_7f3e, 0x7f0e_5f33, 0x5788_8bf5, 0x293d_abf8,
    0x08f9_98f7, 0x764c_b8fa, 0x5eca_6c3c, 0x207f_4c31, 0x0fc7_c5b0, 0x7172_e5bd, 0x59f4_317b, 0x2741_1176,
    0x1a7c_5765, 0x64c9_7768, 0x4c4f_a3ae, 0x32fa_83a3, 0x1d42_0a22, 0x63f7_2a2f, 0x4b71_fee9, 0x35c4_dee4,
    0x1400_edeb, 0x6ab5_cde6, 0x4233_1920, 0x3c86_392d, 0x133e_b0ac, 0x6d8b_90a1, 0x450d_4467, 0x3bb8_646a,
];

#[derive(Debug)]
pub struct Entry {
    offset: Cell<usize>,
    val: usize,
}

pub struct DeltaIndex<'a> {
    _data: &'a [u8],
    hash_mask: usize,
    hash_buckets: Vec<Vec<Rc<Entry>>>,
}

impl<'a> DeltaIndex<'a> {
    pub fn new(data: &[u8]) -> DeltaIndex<'_> {
        // Determine index hash size.  Note that indexing skips the
        // first byte to allow for optimizing the Rabin's polynomial
        // initialization in create_delta().
        // Current delta format can't encode offsets into
        // reference buffer with more than 32 bits.
        let num_entries = (data.len().min(0xFFFF_FFFF) - 1) / RABIN_WINDOW;
        let h_size = num_entries / 4;
        let mut l_shft: usize = 4;
        while (1 << l_shft) < h_size && l_shft < 31 {
            l_shft += 1;
        }
        let h_size = 1 << l_shft;
        //Create fields up front
        let hash_mask = h_size - 1;
        let mut hash_buckets: Vec<Vec<Rc<Entry>>> = vec![vec![]; h_size];
        // Populate the index
        for offset in (0..num_entries * RABIN_WINDOW - RABIN_WINDOW).rev() {
            let mut val: usize = 0;
            for datum in &data[offset + 1..=offset + RABIN_WINDOW] {
                val = (((val << 8) & 0xFFFF_FFFF) | *datum as usize)
                    ^ TANGO[val >> RABIN_SHIFT] as usize;
                let hash_index = val & hash_mask;
                if !hash_buckets[hash_index].is_empty() && hash_buckets[hash_index][0].val == val {
                    // keep the lowest of consecutive identical blocks
                    hash_buckets[hash_index][0]
                        .offset
                        .set(offset + RABIN_WINDOW);
                } else {
                    let entry = Rc::new(Entry {
                        offset: Cell::new(offset + RABIN_WINDOW),
                        val,
                    });
                    hash_buckets[hash_index].insert(0, entry);
                }
            }
        }
        // Determine a limit on the number of entries in the same hash
        // bucket.  This guards us against pathological data sets causing
        // really bad hash distribution with most entries in the same hash
        // bucket that would bring us to O(m*n) computing costs (m and n
        // corresponding to reference and target buffer sizes).
        //
        // Make sure none of the hash buckets has more entries than
        // we're willing to test.  Otherwise we cull the entry list
        // uniformly to still preserve a good repartition across
        // the reference buffer.
        for hash_bucket in hash_buckets.iter_mut() {
            if hash_bucket.len() <= HASH_LIMIT {
                continue;
            }
            // We leave exactly HASH_LIMIT entries in the bucket
            let acc_step = hash_bucket.len() - HASH_LIMIT;
            for index in 1..=HASH_LIMIT {
                let mut acc = acc_step;
                loop {
                    hash_bucket.remove(index);
                    if acc > HASH_LIMIT {
                        acc -= HASH_LIMIT;
                    } else {
                        break;
                    }
                }
            }
            assert_eq!(hash_bucket.len(), HASH_LIMIT)
        }
        DeltaIndex {
            _data: data,
            hash_mask,
            hash_buckets,
        }
    }

    pub fn get_entries(&self, val: usize) -> &Vec<Rc<Entry>> {
        &self.hash_buckets[val & self.hash_mask]
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
            for (mask, lshift) in [0, 1, 2, 3].iter().map(|i| (0x01u8 << i, 8 * i)) {
                if cmd & mask != 0u8 {
                    cp_offset |= (delta[index] as usize) << lshift;
                    index += 1;
                }
            }
            for (mask, lshift) in [0, 1, 2].iter().map(|i| (0x01u8 << i, 8 * i)) {
                if cmd & mask != 0u8 {
                    cp_size |= (delta[index] as usize) << lshift;
                    index += 1;
                }
            }
            if cp_size == 0 {
                cp_size = 0x10000;
            }
            output.extend(source[cp_offset..cp_offset + cp_size].iter());
        } else if cmd != 0 {
            if index > expected_size - output.len() {
                break;
            }
            output.push(delta[index]);
            index += 1;
        } else {
            // cmd == 0 is reserved for future encoding
            // extensions. In the mean time we must fail when
            // encountering them (might be data corruption).
            return Err(DeltaError::PatchError(
                "unexpected delta opcode 0".to_string(),
            ));
        }
    }
    if index < delta.len() || expected_size < output.len() {
        let msg = format!(
            "delta replay has gone wild {0}:{1}:{2}",
            index,
            expected_size,
            output.len()
        );
        return Err(DeltaError::PatchError(msg));
    }
    Ok(output)
}
