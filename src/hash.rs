use crate::H256;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::mem::transmute;

/// merge two hashes
pub fn merge(lhs: &H256, rhs: &H256) -> H256 {
    let mut hash = [0u8; 32];
    let mut counter: usize = 0;
    let mut hasher = DefaultHasher::new();
    hasher.write(lhs);
    hasher.write(rhs);
    let hash64: [u8; 8] = unsafe { transmute(hasher.finish().to_be()) };
    hash64.iter().for_each(|val| {
        hash[counter] = *val;
        counter += 1
    });
    let hash_value = |value| -> [u8; 8] {
        let mut hasher = DefaultHasher::new();
        hasher.write(value);
        unsafe { transmute(hasher.finish().to_be()) }
    };
    loop {
        let hash64 = hash_value(&hash64);
        for val in &hash64 {
            hash[counter] = *val;
            counter += 1;
        }
        if counter == hash.len() {
            return hash;
        }
    }
}
