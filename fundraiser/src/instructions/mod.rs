
use pinocchio::program_error::ProgramError;
use crate::FundraiserInstructions::Finalize;
use crate::instructions::FundraiserInstructions::{Contribute, Initialize, Refund};
mod initialize;
pub use initialize::*;
mod contribute;
pub use contribute::*;
pub mod finalize;
pub use finalize::*;

mod refund;
pub use refund::*;

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
