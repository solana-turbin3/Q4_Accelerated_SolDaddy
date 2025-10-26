use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Contributor {
    pub amount: u64,
}

impl Contributor {
    pub const LEN: usize = core::mem::size_of::<Self>();
    
    #[inline(always)]
    pub unsafe fn from_account_info_unchecked(account_info: &AccountInfo) -> &mut Self {
        &mut *(account_info.borrow_mut_data_unchecked().as_ptr() as *mut Self)
    }
    
    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        if account_info.data_len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { Self::from_account_info_unchecked(account_info) })
    }

    pub fn new(&mut self) {
        self.amount = 0;
    }
}
