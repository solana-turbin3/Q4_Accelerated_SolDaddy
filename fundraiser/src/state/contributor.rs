use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Contributor {
    pub amount: u64,
}

impl Contributor {
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

    pub fn new(&mut self) {
        self.amount = 0;
    }
}
