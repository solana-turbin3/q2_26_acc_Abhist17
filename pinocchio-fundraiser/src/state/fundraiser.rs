

#[repr(C)]
pub struct Fundraiser {
    pub maker: [u8; 32],
       pub mint_to_raise: [u8; 32],
       pub amount_to_raise: [u8; 8],
       pub current_amount: [u8; 8],
       pub time_started: [u8; 8],
       pub duration: u8,
       pub bump: u8,
}

impl Fundraiser {
    pub const LEN: usize = 32 + 32 + 8+8 + 8 + 1+1;

    
}