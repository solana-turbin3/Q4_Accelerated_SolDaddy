mod initialize;

use pinocchio::program_error::ProgramError;
pub use initialize::*;
mod contribute;
pub use contribute::*;
mod refund;
pub mod finalize;
pub use finalize::*;

pub use refund::*;
use crate::FundraiserInstructions::Finalize;
use crate::instructions::FundraiserInstructions::{Contribute, Initialize, Refund};

pub enum FundraiserInstructions {
    Initialize = 0,
    Contribute=1,
    Refund=2,
    Finalize=3,
}

impl TryFrom<&u8> for FundraiserInstructions {
    type Error = ProgramError;

    fn try_from(value: &u8) -> Result<Self, Self::Error> {
        match *value {
            0 => Ok(Initialize),
            1 => Ok(Contribute),
            2 => Ok(Refund),
            3 => Ok(Finalize),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }
}
