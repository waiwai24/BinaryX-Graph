use sha2::{Digest, Sha256};

pub fn generate_string_uid(value: &str) -> String {
    let hash = Sha256::digest(value.as_bytes());
    format!("str:{:x}", hash)
}

pub fn parse_address(address_str: &str) -> Option<u64> {
    let trimmed = address_str.trim();

    if trimmed.is_empty() {
        return None;
    }

    // Try parsing with 0x/0X prefix
    if let Some(hex_str) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex_str, 16).ok();
    }

    // Try parsing pure hexadecimal (a-f characters)
    if trimmed
        .chars()
        .any(|c| c.is_ascii_hexdigit() && !c.is_ascii_digit())
    {
        return u64::from_str_radix(trimmed, 16).ok();
    }

    // Try parsing decimal
    if let Ok(decimal) = trimmed.parse::<u64>() {
        return Some(decimal);
    }

    // Finally try as hexadecimal
    u64::from_str_radix(trimmed, 16).ok()
}

pub fn format_address(address: u64) -> String {
    format!("0x{:x}", address)
}

pub fn normalize_address(address_str: &str) -> Option<String> {
    parse_address(address_str).map(format_address)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_address_hex_prefix() {
        assert_eq!(parse_address("0x1000"), Some(0x1000));
        assert_eq!(parse_address("0X1000"), Some(0x1000));
        assert_eq!(parse_address("0xABCD"), Some(0xABCD));
    }

    #[test]
    fn test_parse_address_decimal() {
        assert_eq!(parse_address("4096"), Some(4096));
        assert_eq!(parse_address("0"), Some(0));
    }

    #[test]
    fn test_parse_address_pure_hex() {
        assert_eq!(parse_address("abcd"), Some(0xABCD));
        assert_eq!(parse_address("ABCD"), Some(0xABCD));
    }

    #[test]
    fn test_parse_address_invalid() {
        assert_eq!(parse_address(""), None);
        assert_eq!(parse_address("   "), None);
        assert_eq!(parse_address("xyz"), None);
    }

    #[test]
    fn test_normalize_address() {
        assert_eq!(normalize_address("0x1000"), Some("0x1000".to_string()));
        assert_eq!(normalize_address("4096"), Some("0x1000".to_string()));
        assert_eq!(normalize_address("0X00001000"), Some("0x1000".to_string()));
    }

    #[test]
    fn test_generate_string_uid() {
        let uid1 = generate_string_uid("Hello");
        let uid2 = generate_string_uid("Hello");
        let uid3 = generate_string_uid("World");

        assert_eq!(uid1, uid2);
        assert_ne!(uid1, uid3);
        assert!(uid1.starts_with("str:"));

        // 验证 SHA256 哈希的长度（64个十六进制字符 + "str:" 前缀）
        assert_eq!(uid1.len(), 4 + 64); // "str:" + 64 chars
        assert_eq!(uid3.len(), 4 + 64);
    }

    #[test]
    fn test_sha256_stability() {
        // 验证特定字符串产生固定的哈希值
        let hello_uid = generate_string_uid("Hello");
        // SHA256("Hello") = 185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969
        assert_eq!(
            hello_uid,
            "str:185f8db32271fe25f561a6fc938b2e264306ec304eda518007d1764826381969"
        );

        let empty_uid = generate_string_uid("");
        // SHA256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            empty_uid,
            "str:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
