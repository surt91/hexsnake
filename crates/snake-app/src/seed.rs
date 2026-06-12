//! Compact seed representation: 32-bit seeds shown as 6 URL-safe base64
//! characters (instead of up to 20 decimal digits).
//!
//! Parsing accepts both forms. This is unambiguous: a u32 occupies the low
//! 32 of the 36 encoded bits, so the first base64 character is always in
//! `A..=P` — an encoded seed can never consist of digits only, while a
//! decimal seed always does.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// Display form of a seed: 6 base64 chars for 32-bit seeds, decimal for
/// legacy seeds that exceed 32 bits.
pub fn encode_seed(seed: u64) -> String {
    let Ok(seed) = u32::try_from(seed) else {
        return seed.to_string();
    };
    let bits = u64::from(seed);
    (0..6)
        .rev()
        .map(|i| ALPHABET[((bits >> (i * 6)) & 0x3F) as usize] as char)
        .collect()
}

/// Parse a seed in either form: decimal (digits only) or 6-char base64.
pub fn parse_seed(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.bytes().all(|b| b.is_ascii_digit()) {
        return s.parse().ok();
    }
    if s.len() != 6 {
        return None;
    }
    let mut bits: u64 = 0;
    for byte in s.bytes() {
        let value = ALPHABET.iter().position(|&a| a == byte)?;
        bits = (bits << 6) | value as u64;
    }
    u32::try_from(bits).ok().map(u64::from)
}

/// A fresh 32-bit seed derived from the wall clock (app side only — the
/// core stays clock-free).
pub fn random_seed() -> u64 {
    entropy() & 0xFFFF_FFFF
}

#[cfg(not(target_arch = "wasm32"))]
fn entropy() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

#[cfg(target_arch = "wasm32")]
fn entropy() -> u64 {
    eframe::web_sys::js_sys::Date::now() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_through_display_form() {
        for seed in [0u64, 1, 42, 0xDEAD_BEEF, u32::MAX as u64] {
            let encoded = encode_seed(seed);
            assert_eq!(encoded.len(), 6, "compact form is 6 chars: {encoded}");
            assert_eq!(parse_seed(&encoded), Some(seed), "roundtrip of {seed}");
        }
    }

    #[test]
    fn encoded_first_char_never_a_digit() {
        // Top 4 bits of the 36-bit group are zero for u32 seeds, so the
        // first char is in A..=P — this is what keeps decimal and base64
        // forms unambiguous.
        for seed in [0u64, 7, 12345, 0xFFFF_FFFF] {
            let first = encode_seed(seed).chars().next().unwrap();
            assert!(('A'..='P').contains(&first), "got {first} for {seed}");
        }
    }

    #[test]
    fn parses_decimal() {
        assert_eq!(parse_seed("42"), Some(42));
        assert_eq!(parse_seed("18446744073709551615"), Some(u64::MAX));
    }

    #[test]
    fn legacy_large_seeds_fall_back_to_decimal_display() {
        let big = u64::MAX;
        assert_eq!(encode_seed(big), big.to_string());
        assert_eq!(parse_seed(&encode_seed(big)), Some(big));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_seed(""), None);
        assert_eq!(parse_seed("zu kurz"), None);
        assert_eq!(parse_seed("!!!!!!"), None);
        assert_eq!(parse_seed("AAAAAAAAAA"), None, "wrong length");
    }

    #[test]
    fn random_seed_fits_in_32_bits() {
        assert!(random_seed() <= u64::from(u32::MAX));
    }
}
