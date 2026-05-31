use pinocchio::{AccountView, error::ProgramError};
use wincode::{SchemaRead, SchemaWrite};

#[derive(SchemaRead, SchemaWrite, Clone, Debug, Default)]
pub struct EscrowV2 {
    pub maker: [u8; 32],
    pub mint_a: [u8; 32],
    pub mint_b: [u8; 32],
    pub amount_to_receive: u64,
    pub amount_to_give: u64,
    pub bump: u8,
}

impl EscrowV2 {
    pub const LEN: usize = 32 + 32 + 32 + 8 + 8 + 1;

    pub fn read_from(account_info: &AccountView) -> Result<Self, ProgramError> {
        let data = account_info.try_borrow()?;
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        wincode::deserialize::<Self>(&data)
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    pub fn write_to(&self, account_info: &AccountView) -> Result<(), ProgramError> {
        let mut data = account_info.try_borrow_mut()?;
        if data.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        let serialized = wincode::serialize(self)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        data[..serialized.len()].copy_from_slice(&serialized);
        Ok(())
    }
}