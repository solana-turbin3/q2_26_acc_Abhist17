use anchor_lang::prelude::*;

declare_id!("77gWVyXhRufiXYer1jF47dCySoScpobpDpZNE3FbDfT7");

mod error;
mod instructions;
mod state;

use instructions::*;

#[program]
pub mod gpt_scheduler {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        process_initialize(ctx)
    }

    pub fn query_gpt(ctx: Context<QueryGpt>) -> Result<()> {
        process_query_gpt(ctx)
    }

    pub fn schedule_query<'info>(
        ctx: Context<'_, '_, 'info, 'info, ScheduleQuery<'info>>,
        task_id: u16,
    ) -> Result<()> {
        process_schedule_query(ctx, task_id)
    }

    pub fn callback_from_gpt(ctx: Context<CallbackFromGpt>, response: String) -> Result<()> {
        process_callback_from_gpt(ctx, response)
    }
}