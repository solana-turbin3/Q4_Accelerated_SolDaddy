use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;


#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Fundraiser {
    pub maker: Pubkey,          // 32 bytes
    pub mint_to_raise: Pubkey,  // 32 bytes
    pub amount_to_raise: u64,   // 8 bytes
    pub current_amount: u64,    // 8 bytes
    pub time_started: i64,      // 8 bytes
    pub duration: u8,           // 1 byte
    pub bump: u8,               // 1 byte
    // total: 90 bytes → padded by compiler to 96 bytes
}

impl Fundraiser {
    pub const LEN: usize = core::mem::size_of::<Self>();

    #[inline(always)]
    pub unsafe fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self)
    }


    pub fn from_account_info(
        account_info: &AccountInfo,
    ) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { Self::from_account_info_unchecked(account_info) })
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
    }
}
