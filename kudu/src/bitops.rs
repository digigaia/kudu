
#[allow(clippy::double_parens)]
pub fn endian_reverse_u32(x: u32) -> u32 {
    (((x >> 24) & 0xFF)      ) |
    (((x >> 16) & 0xFF) <<  8) |
    (((x >>  8) & 0xFF) << 16) |
    (((x      ) & 0xFF) << 24)
}
