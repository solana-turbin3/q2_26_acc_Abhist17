use anchor_lang::prelude::*;

#[account]
pub struct GptScheduler {
    pub context: Pubkey,      // GPT context account
    pub authority: Pubkey,    // Admin who can schedule
    pub query: String,        // The query to ask GPT
    pub last_response: String, // Last response from GPT
    pub query_count: u32,     // Number of queries made
}

impl GptScheduler {
    pub const MAX_QUERY_LEN: usize = 200;
    pub const MAX_RESPONSE_LEN: usize = 500;

    pub fn space(query: &String) -> usize {
        8 + // discriminator
        32 + // context
        32 + // authority  
        4 + query.len() + // query (string)
        4 + Self::MAX_RESPONSE_LEN + // last_response
        4 // query_count
    }
}