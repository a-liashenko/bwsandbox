const RAND_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_";

pub fn rand_id(len: usize) -> String {
    use rustix::rand::{GetRandomFlags, getrandom};

    let mut bytes = vec![0u8; len];
    getrandom(&mut bytes, GetRandomFlags::empty()).expect("Random not ready");

    for el in &mut bytes {
        *el = RAND_ALPHABET[*el as usize % RAND_ALPHABET.len()];
    }
    String::from_utf8(bytes).expect("Alphabet must be utf8 compatible")
}

#[test]
fn test_rand_id() {
    let size = 32;
    let id = rand_id(size);
    assert_eq!(id.len(), size);

    for ch in id.chars() {
        assert!(RAND_ALPHABET.contains(&(ch as u8)));
    }
}
