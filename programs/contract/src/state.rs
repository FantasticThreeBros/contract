use anchor_lang::prelude::*;


#[account]
#[derive(InitSpace)]
pub struct RegistryConfig {
    pub admin : Pubkey,
    pub is_frozen : bool,
    pub merkle_root : [u8;32],
    pub tree_depth : u8,
    pub bump : u8   
}

#[account]
#[derive(InitSpace)]
pub struct VerifierConfig {
    pub verifier_key : Pubkey ,
    pub is_active : bool ,
    pub bump : u8
}