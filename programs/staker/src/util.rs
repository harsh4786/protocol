use std::convert::TryInto;

use anchor_lang::prelude::*;

pub const STAKER_SEED: &str = "staker";

pub fn check_position_seeds<'info>(
    owner: AccountInfo<'info>,
    position: &Pubkey,
    index: u32,
) -> bool {
    &Pubkey::find_program_address(
        &[b"positionv1", owner.key.as_ref(), &index.to_le_bytes()],
        &amm::program::Amm::id(),
    )
    .0 == position
}

pub fn get_current_timestamp() -> u64 {
    Clock::get().unwrap().unix_timestamp.try_into().unwrap()
}

pub fn get_current_slot() -> u64 {
    Clock::get().unwrap().slot
}
