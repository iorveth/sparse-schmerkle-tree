mod error;
mod hash;

use error::{Error, Result};
use hash::merge;
use std::borrow::Cow;
use std::collections::HashMap;

pub type H256 = [u8; 32];
pub type TreeCache = HashMap<H256, (H256, H256)>;
/// leaves default hash
pub const ZERO_HASH: H256 = [0u8; 32];
const TREE_HEIGHT: usize = std::mem::size_of::<H256>() * 8;
const HIGHEST_BIT_POS: u8 = 7;

#[derive(Debug, PartialEq)]
pub enum Branch {
    Left = 0,
    Right = 1,
}

/// H256 path iterator
/// iterate from left to right, from higher bit to lower bit.
struct PathIter<'a> {
    path: &'a H256,
    bit_pos: u8,
    byte_pos: u8,
}

impl<'a> From<&'a H256> for PathIter<'a> {
    fn from(path: &'a H256) -> Self {
        PathIter {
            path,
            bit_pos: 0,
            byte_pos: 0,
        }
    }
}

impl<'a> Iterator for PathIter<'a> {
    type Item = Branch;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(byte) = self.path.get(self.byte_pos as usize) {
            let branch = if (byte >> (HIGHEST_BIT_POS - self.bit_pos)) & 1 == 1 {
                Branch::Right
            } else {
                Branch::Left
            };
            if self.bit_pos == HIGHEST_BIT_POS {
                self.byte_pos += 1;
                self.bit_pos = 0;
            } else {
                self.bit_pos += 1;
            }
            Some(branch)
        } else {
            None
        }
    }
}

/// Sparse merkle tree
#[derive(Clone, Debug)]
pub struct SparseMerkleTree {
    pub cache: TreeCache,
    pub root: H256,
}

impl SparseMerkleTree {
    /// create merkle tree from root and cache
    pub fn new(root: H256, cache: TreeCache) -> SparseMerkleTree {
        SparseMerkleTree { root, cache }
    }

    pub fn compute_default_tree() -> SparseMerkleTree {
        let mut hash = ZERO_HASH;
        let mut cache: TreeCache = Default::default();
        for _ in 0..256 {
            let parent = merge(&hash, &hash);
            cache.insert(parent, (hash, hash));
            hash = parent;
        }
        SparseMerkleTree::new(hash, cache)
    }

    /// add or update leaf value.
    pub fn insert(&mut self, key: &H256, value: H256) -> Result<()> {
        let mut node = &self.root;
        let mut siblings = Vec::with_capacity(256);
        for branch in PathIter::from(key) {
            let parent = self.cache.get(node).ok_or(Error::MissingKey(*node))?;
            match branch {
                Branch::Left => {
                    siblings.push(parent.1);
                    node = &parent.0;
                }
                Branch::Right => {
                    siblings.push(parent.0);
                    node = &parent.1;
                }
            }
        }
        let mut node = value;
        for branch in PathIter::from(key).collect::<Vec<_>>().into_iter().rev() {
            let sibling = siblings.pop().expect("sibling should exsits");
            match branch {
                Branch::Left => {
                    let new_parent = merge(&node, &sibling);
                    self.cache.insert(new_parent, (node, sibling));
                    node = new_parent;
                }
                Branch::Right => {
                    let new_parent = merge(&sibling, &node);
                    self.cache.insert(new_parent, (sibling, node));
                    node = new_parent;
                }
            }
        }
        self.root = node;
        Ok(())
    }

    /// get leaf value. If value isn`t null, also return it`s merkle path.
    pub fn get<'a>(&self, key: &'a H256) -> Result<(&H256, Option<Vec<Branch>>)> {
        let mut node = &self.root;
        let mut path = vec![];
        for branch in PathIter::from(key) {
            let parent = self.cache.get(node).ok_or(Error::MissingKey(*node))?;
            match branch {
                Branch::Left => node = &parent.0,
                Branch::Right => node = &parent.1,
            }
            path.push(branch);
        }
        if *node != ZERO_HASH {
            Ok((node, Some(path)))
        } else {
            Ok((node, None))
        }
    }

    /// generate merkle proof
    fn merkle_proof(&self, path: &[Branch]) -> Result<Vec<H256>> {
        let mut node = &self.root;
        let mut proof = Vec::with_capacity(256);
        for branch in path {
            let parent = self.cache.get(node).ok_or(Error::MissingKey(*node))?;
            match branch {
                Branch::Left => {
                    proof.push(parent.1);
                    node = &parent.0;
                }
                Branch::Right => {
                    proof.push(parent.0);
                    node = &parent.1;
                }
            }
        }
        Ok(proof)
    }

    /// verify merkle path provided.
    pub fn verify(&self, value: &H256, path: &[Branch]) -> Result<bool> {
        let proof = self.merkle_proof(path)?;
        if proof.len() != TREE_HEIGHT {
            return Ok(false);
        }
        let mut node = Cow::Borrowed(value);
        for (i, branch) in path.into_iter().rev().enumerate() {
            let sibling = match proof.get(TREE_HEIGHT - i - 1) {
                Some(sibling) => sibling,
                None => {
                    return Ok(false);
                }
            };
            match branch {
                Branch::Left => {
                    node = Cow::Owned(merge(node.as_ref(), sibling));
                }
                Branch::Right => {
                    node = Cow::Owned(merge(sibling, node.as_ref()));
                }
            }
        }
        Ok(&self.root == node.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default_root() {
        let tree = SparseMerkleTree::compute_default_tree();
        assert_eq!(tree.cache.len(), 256);
        assert_eq!(
            tree.root,
            [
                140, 164, 124, 238, 105, 175, 51, 44, 10, 239, 182, 210, 7, 199, 111, 54, 10, 239,
                182, 210, 7, 199, 111, 54, 10, 239, 182, 210, 7, 199, 111, 54
            ]
        );
    }

    #[test]
    fn test_insert() {
        let mut tree = SparseMerkleTree::compute_default_tree();
        let key = [
            242, 160, 143, 147, 201, 240, 57, 245, 126, 181, 190, 235, 95, 42, 240, 169, 94, 190,
            197, 240, 67, 46, 153, 190, 244, 230, 180, 164, 230, 230, 230, 240,
        ];
        let value = [
            159, 152, 25, 88, 11, 146, 36, 220, 86, 143, 224, 156, 103, 44, 183, 6, 156, 89, 22,
            120, 236, 205, 174, 144, 138, 191, 158, 229, 217, 64, 152, 245,
        ];
        let (val1, path1) = tree.get(&key).expect("get");
        assert_eq!((val1, path1), (&ZERO_HASH, None));
        tree.insert(&key, value).expect("insert");
        let (val2, path2) = tree.get(&key).expect("get");
        assert!(val2 == &value && path2.is_some());
    }

    #[test]
    fn test_verify() {
        let mut tree = SparseMerkleTree::compute_default_tree();
        let key = [
            77, 160, 178, 147, 201, 240, 57, 245, 126, 181, 190, 235, 95, 42, 240, 169, 94, 190,
            197, 240, 67, 46, 153, 190, 244, 230, 180, 164, 230, 230, 66, 240,
        ];
        let value = [
            159, 89, 45, 88, 11, 146, 36, 220, 86, 143, 224, 156, 103, 44, 183, 6, 156, 89, 22,
            120, 236, 205, 174, 144, 138, 191, 190, 229, 217, 64, 152, 30,
        ];
        tree.insert(&key, value).expect("insert");
        let (_, path) = tree.get(&key).expect("get");
        assert!(tree.verify(&value, &path.expect("path")).expect("verify"));
    }

    #[test]
    #[should_panic]
    fn test_verify_should_panic() {
        let tree = SparseMerkleTree::compute_default_tree();
        let value = [
            77, 160, 178, 147, 201, 240, 57, 245, 126, 181, 190, 235, 95, 42, 240, 169, 94, 190,
            197, 240, 67, 46, 153, 190, 244, 230, 180, 164, 230, 230, 66, 240,
        ];
        let path = vec![Branch::Left, Branch::Right, Branch::Left];
        assert!(tree.verify(&value, &path).expect("verify"));
    }
}
