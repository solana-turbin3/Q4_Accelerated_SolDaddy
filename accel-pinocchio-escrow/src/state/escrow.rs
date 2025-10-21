use pinocchio::{account_info::AccountInfo, program_error::ProgramError};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Escrow {
    maker: [u8; 32],
    mint_a: [u8; 32],
    mint_b: [u8; 32],
    amount_to_receive: [u8; 8],
    amount_to_give: [u8; 8],
    pub bump: u8,
}

impl Escrow {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 1; //Bump was not taken into account

    pub fn from_account_info(account_info: &AccountInfo) -> Result<&mut Self, ProgramError> {
        let mut data = account_info.try_borrow_mut_data()?;
        if data.len() != Escrow::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if (data.as_ptr() as usize) % core::mem::align_of::<Self>() != 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        // let mut escrow = Escrow::default();
        
        // escrow.maker.copy_from_slice(&data[0..32]);
        // escrow.mint_a.copy_from_slice(&data[32..64]);
        // escrow.mint_b.copy_from_slice(&data[64..96]);
        // escrow.amount_to_receive.copy_from_slice(&data[96..104]);

        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }

    pub fn maker(&self) -> pinocchio::pubkey::Pubkey {
        pinocchio::pubkey::Pubkey::from(self.maker)
    }

    pub fn set_maker(&mut self, maker: &pinocchio::pubkey::Pubkey) {
        self.maker.copy_from_slice(maker.as_ref());
    }

    pub fn mint_a(&self) -> pinocchio::pubkey::Pubkey {
        pinocchio::pubkey::Pubkey::from(self.mint_a)
    }

    pub fn set_mint_a(&mut self, mint_a: &pinocchio::pubkey::Pubkey) {
        self.mint_a.copy_from_slice(mint_a.as_ref());
    }

    pub fn mint_b(&self) -> pinocchio::pubkey::Pubkey {
        pinocchio::pubkey::Pubkey::from(self.mint_b)
    }

    pub fn set_mint_b(&mut self, mint_b: &pinocchio::pubkey::Pubkey) {
        self.mint_b.copy_from_slice(mint_b.as_ref());
    }

    pub fn amount_to_receive(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_receive)
    }

    pub fn set_amount_to_receive(&mut self, amount: u64) {
        self.amount_to_receive = amount.to_le_bytes();
    }

    pub fn amount_to_give(&self) -> u64 {
        u64::from_le_bytes(self.amount_to_give)
    }

    pub fn set_amount_to_give(&mut self, amount: u64) {
        self.amount_to_give = amount.to_le_bytes();
    }
}