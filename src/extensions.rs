pub trait StartsWithIgnoreCase {
    fn starts_with_i(&self, pre: &str) -> bool;
}

impl StartsWithIgnoreCase for String {
    fn starts_with_i(&self, other: &str) -> bool {
        self.get(..other.len())
            .map(|x| x.eq_ignore_ascii_case(other))
            .unwrap_or(false)
    }
}

impl StartsWithIgnoreCase for &str {
    fn starts_with_i(&self, other: &str) -> bool {
        self.get(..other.len())
            .map(|x| x.eq_ignore_ascii_case(other))
            .unwrap_or(false)
    }
}

#[test]
fn test_starts_with_i() {
    assert!("HuEhuUehEheUeIiIAAAA".starts_with_i("huehuueheheueiii"));
    assert!(!"xdxdxd".starts_with_i("huehuueheheueiii"));
}
