use cosmwasm_std::{StdError, Uint128};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("Bet too small, minimum is {min}")]
    BetTooSmall { min: Uint128 },

    #[error("Game not available")]
    GameNotAvailable {},

    #[error("Game timeout")]
    GameTimeout {},

    #[error("Cannot join own game")]
    CannotJoinOwnGame {},
}
