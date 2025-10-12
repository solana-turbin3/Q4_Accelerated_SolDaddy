use anchor_lang::prelude::*;

// #[account]
// pub struct Whitelist {
//     pub address: Vec<Pubkey>,
//     pub bump: u8,
// }


#[account]
pub struct Whitelist {
    /// The user that is whitelisted for this mint
    pub user: Pubkey,
    /// The mint this whitelist entry is tied to
    pub mint: Pubkey,
    pub bump: u8,
}