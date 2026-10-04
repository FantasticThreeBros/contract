use anchor_lang::prelude::*;

#[constant]
pub const COUNTER_SEED: &[u8] = b"counter";

#[constant]
pub const HELLO_WORLD_LAMPORTS: u64 = 1;

#[constant]
pub const MAX_COUNT: u64 = 10;

#[constant]
pub const REGISTRY_SEED: &[u8] = b"registry";

#[constant]
pub const VERIFIER_CONFIG_SEED: &[u8] = b"verifier_config";

#[constant]
pub const IDENTITY_SEED: &[u8] = b"identity";

#[constant]
pub const NULLIFIER_SEED: &[u8] = b"nullifier";

pub const TREE_DEPTH: u8 = 20;
