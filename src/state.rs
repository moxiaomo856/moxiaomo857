use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub admin: Addr,
    pub momo_token: Addr,
    pub fee_percentage: u64,
    pub min_bet_paxi: Uint128,
    pub min_bet_momo: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Game {
    pub creator: Addr,
    pub joiner: Option<Addr>,
    pub bet_paxi: Uint128,
    pub bet_momo: Uint128,
    pub creator_choice: Option<u8>,
    pub joiner_choice: Option<u8>,
    pub status: GameStatus,
    pub created_at: u64,
    pub timeout_at: u64,
    pub winner: Option<Addr>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GameStatus {
    Waiting,
    Joined,
    Completed,
    Timeout,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const GAMES: Map<u64, Game> = Map::new("games");
pub const GAME_COUNTER: Item<u64> = Item::new("game_counter");
