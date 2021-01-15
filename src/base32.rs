use num_bigint::BigUint;
pub fn encode(bytes: &[u8]) -> String {
    let num = BigUint::from_bytes_le(bytes);
    num.to_str_radix(32)
}
