use anchor_lang::prelude::*;

#[constant]
pub const REGISTRY_SEED: &[u8] = b"registry";

#[constant]
pub const VERIFIER_CONFIG_SEED: &[u8] = b"verifier_config";

#[constant]
pub const ENROLLMENT_SEED: &[u8] = b"enrollment";

#[constant]
pub const NULLIFIER_SEED: &[u8] = b"nullifier";

pub const TREE_DEPTH: u8 = 20;
