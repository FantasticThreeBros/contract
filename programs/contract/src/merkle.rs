//! Fixed-depth sparse Merkle tree helpers (sha256, used consistently for both
//! commitments and tree nodes).
//!
//! POC SIMPLIFICATION: the full tree lives off-chain with the client. The chain
//! stores only the root and the next free leaf index. Because the tree is
//! append-only, the siblings along the path of the next free leaf are enough to
//! (a) prove the old root had an empty leaf there and (b) recompute the new
//! root with the commitment inserted. Concurrent enrolls invalidate each
//! other's proofs (the client must re-fetch and retry); a production design
//! would use a concurrent Merkle tree (e.g. SPL account-compression).

use anchor_lang::prelude::*;
use solana_sha256_hasher::hashv;

use crate::error::PassportError;

pub type Node = [u8; 32];

pub const EMPTY_LEAF: Node = [0u8; 32];

pub fn hash_pair(left: &Node, right: &Node) -> Node {
    hashv(&[left, right]).to_bytes()
}

/// commitment = sha256(credential account pubkey)
pub fn commitment_from_credential(credential: &Pubkey) -> Node {
    hashv(&[credential.as_ref()]).to_bytes()
}

/// Root of an entirely empty tree of the given depth.
pub fn empty_root(depth: u8) -> Node {
    let mut node = EMPTY_LEAF;
    for _ in 0..depth {
        node = hash_pair(&node, &node);
    }
    node
}

/// Fold a leaf up to the root along `siblings` (bottom-up) for `index`.
pub fn compute_root(leaf: Node, index: u32, siblings: &[Node]) -> Node {
    let mut node = leaf;
    let mut idx = index;
    for sibling in siblings {
        node = if idx & 1 == 0 {
            hash_pair(&node, sibling)
        } else {
            hash_pair(sibling, &node)
        };
        idx >>= 1;
    }
    node
}

pub fn check_proof_len(depth: u8, proof: &[Node]) -> Result<()> {
    require!(proof.len() == depth as usize, PassportError::InvalidProofLength);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_flow_matches_full_tree() {
        let depth = 3u8;
        let mut root = empty_root(depth);
        let mut leaves = [EMPTY_LEAF; 8];
        for i in 0..8u32 {
            // Build siblings for index i from the full tree.
            let mut level: Vec<Node> = leaves.to_vec();
            let mut idx = i as usize;
            let mut siblings = vec![];
            for _ in 0..depth {
                siblings.push(level[idx ^ 1]);
                level = level.chunks(2).map(|c| hash_pair(&c[0], &c[1])).collect();
                idx >>= 1;
            }
            assert_eq!(compute_root(EMPTY_LEAF, i, &siblings), root);
            let c = commitment_from_credential(&Pubkey::new_unique());
            root = compute_root(c, i, &siblings);
            leaves[i as usize] = c;
            assert_eq!(compute_root(c, i, &siblings), root);
        }
    }
}
