use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Fundraiser {
    pub maker: Pubkey,          // 32 bytes
    pub mint_to_raise: Pubkey,  // 32 bytes
    pub amount_to_raise: u64,   // 8 bytes
    pub current_amount: u64,    // 8 bytes
    pub time_started: i64,      // 8 bytes
    pub duration: u8,           // 1 byte
    pub bump: u8,               // 1 byte
    pub _padding: [u8; 6],
}

impl Fundraiser {
    pub const LEN: usize = core::mem::size_of::<Self>();


    #[inline(always)]
    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        let data = unsafe { account_info.borrow_mut_data_unchecked() };
        bytemuck::try_from_bytes_mut(data)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn new(
        &mut self,
        maker: &Pubkey,
        mint_to_raise: &Pubkey,
        amount_to_raise: u64,
        time_started: i64,
        duration: u8,
        bump: u8,
    ) {
        self.maker = *maker;
        self.mint_to_raise = *mint_to_raise;
        self.amount_to_raise = amount_to_raise;
        self.current_amount = 0;
        self.time_started = time_started;
        self.duration = duration;
        self.bump = bump;
        self._padding = [0; 6];
    }
}
