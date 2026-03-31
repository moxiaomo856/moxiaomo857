use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub momo_token: String,           // SHV代币地址
    pub fee_percentage: u64,          // 手续费百分比
    pub min_bet_paxi: Uint128,        // 最小PAXI赌注
    pub min_bet_momo: Uint128,        // 最小SHV赌注
}

#[cw_serde]
pub enum ExecuteMsg {
    CreateGame {
        bet_paxi: Uint128,
        bet_momo: Uint128,
    },
    JoinGame {
        game_id: u64,
        choice: u8,  // 0=石头, 1=布, 2=剪刀
    },
    Reveal {
        game_id: u64,
        choice: u8,
        salt: String,
    },
    ClaimTimeout {
        game_id: u64,
    },
    UpdateConfig {
        fee_percentage: Option<u64>,
        min_bet_paxi: Option<Uint128>,
        min_bet_momo: Option<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GameInfo)]
    GetGame { game_id: u64 },
    
    #[returns(Config)]
    GetConfig {},
    
    #[returns(Vec<GameInfo>)]
    GetGames { limit: Option<u32>, start_after: Option<u64> },
}

#[cw_serde]
pub struct GameInfo {
    pub game_id: u64,
    pub creator: Addr,
    pub joiner: Option<Addr>,
    pub bet_paxi: Uint128,
    pub bet_momo: Uint128,
    pub status: GameStatus,
    pub created_at: u64,
    pub timeout_at: u64,
    pub winner: Option<Addr>,
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub momo_token: Addr,
    pub fee_percentage: u64,
    pub min_bet_paxi: Uint128,
    pub min_bet_momo: Uint128,
}

#[cw_serde]
pub enum GameStatus {
    Waiting,
    Joined,
    Completed,
    Timeout,
}
