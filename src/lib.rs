use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, 
    Response, StdResult, Uint128, Addr, Coin, BankMsg, WasmMsg,
    StdError
};
use cw2::set_contract_version;
use cw20::Cw20ExecuteMsg;

use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, GameInfo, GameStatus, Config as ConfigMsg};
use crate::state::{Config, Game, CONFIG, GAMES, GAME_COUNTER};
use crate::error::ContractError;

// 版本信息
const CONTRACT_NAME: &str = "crates.io:rps-game";
const CONTRACT_VERSION: &str = "0.1.0";

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    
    // 验证代币地址
    let momo_token = deps.api.addr_validate(&msg.momo_token)?;
    
    // 保存配置
    let config = Config {
        admin: info.sender.clone(),
        momo_token: momo_token.clone(),
        fee_percentage: msg.fee_percentage,
        min_bet_paxi: msg.min_bet_paxi,
        min_bet_momo: msg.min_bet_momo,
    };
    CONFIG.save(deps.storage, &config)?;
    
    // 初始化游戏计数器
    GAME_COUNTER.save(deps.storage, &0u64)?;
    
    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", info.sender)
        .add_attribute("momo_token", momo_token))
}

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateGame { bet_paxi, bet_momo } => {
            execute_create_game(deps, env, info, bet_paxi, bet_momo)
        }
        ExecuteMsg::JoinGame { game_id, choice } => {
            execute_join_game(deps, env, info, game_id, choice)
        }
        ExecuteMsg::Reveal { game_id, choice, salt: _ } => {
            execute_reveal(deps, env, info, game_id, choice)
        }
        ExecuteMsg::ClaimTimeout { game_id } => {
            execute_claim_timeout(deps, env, info, game_id)
        }
        ExecuteMsg::UpdateConfig {
            fee_percentage,
            min_bet_paxi,
            min_bet_momo,
        } => execute_update_config(deps, info, fee_percentage, min_bet_paxi, min_bet_momo),
    }
}

// 创建游戏
pub fn execute_create_game(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bet_paxi: Uint128,
    bet_momo: Uint128,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    
    // 检查最小赌注
    if bet_paxi < config.min_bet_paxi {
        return Err(ContractError::BetTooSmall { min: config.min_bet_paxi });
    }
    if bet_momo < config.min_bet_momo {
        return Err(ContractError::BetTooSmall { min: config.min_bet_momo });
    }
    
    // 检查PAXI支付
    let sent_paxi = info
        .funds
        .iter()
        .find(|c| c.denom == "upaxi")
        .ok_or(ContractError::InsufficientFunds {})?;
    
    if sent_paxi.amount < bet_paxi {
        return Err(ContractError::InsufficientFunds {});
    }
    
    // 转移MOMO代币到合约
    let transfer_momo_msg = WasmMsg::Execute {
        contract_addr: config.momo_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: bet_momo,
        })?,
        funds: vec![],
    };
    
    // 生成游戏ID
    let game_id = GAME_COUNTER.update(deps.storage, |id| Ok::<_, ContractError>(id + 1))?;
    
    // 保存游戏信息
    let game = Game {
        creator: info.sender.clone(),
        joiner: None,
        bet_paxi,
        bet_momo,
        creator_choice: None,
        joiner_choice: None,
        status: GameStatus::Waiting,
        created_at: env.block.time.seconds(),
        timeout_at: env.block.time.seconds() + 3600, // 1小时超时
        winner: None,
    };
    
    GAMES.save(deps.storage, game_id, &game)?;
    
    Ok(Response::new()
        .add_message(transfer_momo_msg)
        .add_attribute("action", "create_game")
        .add_attribute("game_id", game_id.to_string())
        .add_attribute("creator", info.sender)
        .add_attribute("bet_paxi", bet_paxi)
        .add_attribute("bet_momo", bet_momo))
}

// 加入游戏
pub fn execute_join_game(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    game_id: u64,
    choice: u8,
) -> Result<Response, ContractError> {
    let mut game = GAMES.load(deps.storage, game_id)?;
    
    // 检查游戏状态
    if game.status != GameStatus::Waiting {
        return Err(ContractError::GameNotAvailable {});
    }
    
    // 检查超时
    if env.block.time.seconds() > game.timeout_at {
        game.status = GameStatus::Timeout;
        GAMES.save(deps.storage, game_id, &game)?;
        return Err(ContractError::GameTimeout {});
    }
    
    // 检查不能加入自己的游戏
    if game.creator == info.sender {
        return Err(ContractError::CannotJoinOwnGame {});
    }
    
    // 检查PAXI支付
    let sent_paxi = info
        .funds
        .iter()
        .find(|c| c.denom == "upaxi")
        .ok_or(ContractError::InsufficientFunds {})?;
    
    if sent_paxi.amount < game.bet_paxi {
        return Err(ContractError::InsufficientFunds {});
    }
    
    // 转移MOMO代币
    let config = CONFIG.load(deps.storage)?;
    let transfer_momo_msg = WasmMsg::Execute {
        contract_addr: config.momo_token.to_string(),
        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
            owner: info.sender.to_string(),
            recipient: env.contract.address.to_string(),
            amount: game.bet_momo,
        })?,
        funds: vec![],
    };
    
    // 更新游戏状态
    game.joiner = Some(info.sender.clone());
    game.joiner_choice = Some(choice);
    game.status = GameStatus::Joined;
    
    GAMES.save(deps.storage, game_id, &game)?;
    
    Ok(Response::new()
        .add_message(transfer_momo_msg)
        .add_attribute("action", "join_game")
        .add_attribute("game_id", game_id.to_string())
        .add_attribute("joiner", info.sender)
        .add_attribute("choice", choice.to_string()))
}

// 揭示选择并结算
pub fn execute_reveal(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    game_id: u64,
    choice: u8,
) -> Result<Response, ContractError> {
    let mut game = GAMES.load(deps.storage, game_id)?;
    
    // 检查游戏状态
    if game.status != GameStatus::Joined {
        return Err(ContractError::GameNotAvailable {});
    }
    
    // 检查超时
    if env.block.time.seconds() > game.timeout_at {
        game.status = GameStatus::Timeout;
        GAMES.save(deps.storage, game_id, &game)?;
        return Err(ContractError::GameTimeout {});
    }
    
    // 验证揭示者
    if info.sender != game.creator {
        return Err(ContractError::Unauthorized {});
    }
    
    // 验证选择
    game.creator_choice = Some(choice);
    
    // 计算胜负
    let joiner_choice = game.joiner_choice.ok_or_else(|| ContractError::GameNotAvailable {})?;
    let winner = determine_winner(choice, joiner_choice, &game.creator, &game.joiner.unwrap());
    
    // 分配奖金
    let config = CONFIG.load(deps.storage)?;
    let total_paxi = game.bet_paxi.checked_mul(Uint128::from(2u64))?;
    let total_momo = game.bet_momo.checked_mul(Uint128::from(2u64))?;
    
    // 计算手续费
    let fee_paxi = total_paxi.multiply_ratio(config.fee_percentage, 100u64);
    let fee_momo = total_momo.multiply_ratio(config.fee_percentage, 100u64);
    
    let prize_paxi = total_paxi.checked_sub(fee_paxi)?;
    let prize_momo = total_momo.checked_sub(fee_momo)?;
    
    // 更新游戏状态
    game.status = GameStatus::Completed;
    game.winner = winner.clone();
    
    GAMES.save(deps.storage, game_id, &game)?;
    
    // 构建返回消息
    let mut response = Response::new()
        .add_attribute("action", "reveal")
        .add_attribute("game_id", game_id.to_string())
        .add_attribute("winner", winner.map(|w| w.to_string()).unwrap_or_else(|| "draw".to_string()));
    
    // 发送奖金给赢家
    if let Some(winner_addr) = winner {
        // 发送PAXI
        if prize_paxi > Uint128::zero() {
            let send_paxi_msg = BankMsg::Send {
                to_address: winner_addr.to_string(),
                amount: vec![Coin {
                    denom: "upaxi".to_string(),
                    amount: prize_paxi,
                }],
            };
            response = response.add_message(send_paxi_msg);
        }
        
        // 发送MOMO
        if prize_momo > Uint128::zero() {
            let send_momo_msg = WasmMsg::Execute {
                contract_addr: config.momo_token.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: winner_addr.to_string(),
                    amount: prize_momo,
                })?,
                funds: vec![],
            };
            response = response.add_message(send_momo_msg);
        }
    } else {
        // 平局，退回赌注
        let return_paxi_msg = BankMsg::Send {
            to_address: game.creator.to_string(),
            amount: vec![Coin {
                denom: "upaxi".to_string(),
                amount: game.bet_paxi,
            }],
        };
        response = response.add_message(return_paxi_msg);
        
        if let Some(joiner) = &game.joiner {
            let return_paxi_msg2 = BankMsg::Send {
                to_address: joiner.to_string(),
                amount: vec![Coin {
                    denom: "upaxi".to_string(),
                    amount: game.bet_paxi,
                }],
            };
            response = response.add_message(return_paxi_msg2);
        }
    }
    
    Ok(response)
}

// 判断胜负
fn determine_winner(choice1: u8, choice2: u8, creator: &Addr, joiner: &Addr) -> Option<Addr> {
    // 0=石头, 1=布, 2=剪刀
    match (choice1, choice2) {
        (0, 2) | (1, 0) | (2, 1) => Some(creator.clone()), // 创建者赢
        (2, 0) | (0, 1) | (1, 2) => Some(joiner.clone()),  // 加入者赢
        _ => None, // 平局
    }
}

// 认领超时游戏
pub fn execute_claim_timeout(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    game_id: u64,
) -> Result<Response, ContractError> {
    let mut game = GAMES.load(deps.storage, game_id)?;
    
    // 检查游戏状态
    if game.status != GameStatus::Timeout {
        return Err(ContractError::GameNotAvailable {});
    }
    
    // 只有创建者可以认领
    if info.sender != game.creator {
        return Err(ContractError::Unauthorized {});
    }
    
    // 退回赌注给创建者
    let return_paxi_msg = BankMsg::Send {
        to_address: game.creator.to_string(),
        amount: vec![Coin {
            denom: "upaxi".to_string(),
            amount: game.bet_paxi,
        }],
    };
    
    game.status = GameStatus::Completed;
    GAMES.save(deps.storage, game_id, &game)?;
    
    Ok(Response::new()
        .add_message(return_paxi_msg)
        .add_attribute("action", "claim_timeout")
        .add_attribute("game_id", game_id.to_string()))
}

// 更新配置
pub fn execute_update_config(
    deps: DepsMut,
    info: MessageInfo,
    fee_percentage: Option<u64>,
    min_bet_paxi: Option<Uint128>,
    min_bet_momo: Option<Uint128>,
) -> Result<Response, ContractError> {
    let mut config = CONFIG.load(deps.storage)?;
    
    // 检查权限
    if info.sender != config.admin {
        return Err(ContractError::Unauthorized {});
    }
    
    if let Some(percent) = fee_percentage {
        if percent > 100 {
            return Err(ContractError::Std(StdError::generic_err("Fee percentage cannot exceed 100")));
        }
        config.fee_percentage = percent;
    }
    
    if let Some(min) = min_bet_paxi {
        config.min_bet_paxi = min;
    }
    
    if let Some(min) = min_bet_momo {
        config.min_bet_momo = min;
    }
    
    CONFIG.save(deps.storage, &config)?;
    
    Ok(Response::new()
        .add_attribute("action", "update_config")
        .add_attribute("fee_percentage", config.fee_percentage.to_string())
        .add_attribute("min_bet_paxi", config.min_bet_paxi)
        .add_attribute("min_bet_momo", config.min_bet_momo))
}

#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetGame { game_id } => to_binary(&query_game(deps, game_id)?),
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetGames { limit: _, start_after: _ } => {
            to_binary(&query_games(deps)?)
        }
    }
}

fn query_game(deps: Deps, game_id: u64) -> StdResult<GameInfo> {
    let game = GAMES.load(deps.storage, game_id)?;
    Ok(GameInfo {
        game_id,
        creator: game.creator,
        joiner: game.joiner,
        bet_paxi: game.bet_paxi,
        bet_momo: game.bet_momo,
        status: game.status,
        created_at: game.created_at,
        timeout_at: game.timeout_at,
        winner: game.winner,
    })
}

fn query_config(deps: Deps) -> StdResult<ConfigMsg> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigMsg {
        admin: config.admin,
        momo_token: config.momo_token,
        fee_percentage: config.fee_percentage,
        min_bet_paxi: config.min_bet_paxi,
        min_bet_momo: config.min_bet_momo,
    })
}

fn query_games(deps: Deps) -> StdResult<Vec<GameInfo>> {
    let mut games = Vec::new();
    for item in GAMES.range(deps.storage, None, None, cosmwasm_std::Order::Ascending) {
        let (id, game) = item?;
        games.push(GameInfo {
            game_id: id,
            creator: game.creator,
            joiner: game.joiner,
            bet_paxi: game.bet_paxi,
            bet_momo: game.bet_momo,
            status: game.status,
            created_at: game.created_at,
            timeout_at: game.timeout_at,
            winner: game.winner,
        });
    }
    Ok(games)
}
