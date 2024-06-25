use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BankMsg, Coin, Coins, Deps, MessageInfo, Response, StdError};
use crate::utility::Validate;
use crate::{Res, Error};

#[cw_serde]
pub struct Claim {
    pub owner: Addr,
    pub bps: u32,
}

impl Claim {
    pub fn split(&self, funds: &Coins) -> Res<Coins> {
        funds.into_iter()
            .map(|coin| Coin::new(coin.amount.u128() * self.bps as u128 / 10000u128, &coin.denom))
            .collect::<Vec<Coin>>()
            .try_into()
            .map_err(|e| StdError::from(e).into())
    }
}

#[cw_serde]
pub struct Split {
    pub claims: Vec<Claim>,
}

impl Split {
    pub fn split(&self, funds: &Coins) -> Res<Vec<BankMsg>> {
        let mut amounts = self.claims.iter()
            .map(|claim| {
                let coins = claim.split(&funds);
                coins.map(|c| (&claim.owner, c))
            })
            .collect::<Res<Vec<(&Addr, Coins)>>>()?;
        let mut totals = Coins::default();
        for (_, coins) in amounts.iter() {
            for coin in coins.clone() {
                totals.add(coin)?;
            }
        }
        let mut remainders = funds.clone();
        for total in totals {
            remainders.sub(total)?;
        };
        for remainder in remainders {
            amounts[0].1.add(remainder)?;
        };
        Ok(amounts.iter()
            .map(|(owner, coins)| {
                BankMsg::Send {
                    to_address: owner.to_string(),
                    amount: coins.to_vec(),
                }
            })
            .collect()
        )
    }
}

pub trait MessageFunds {
    fn require_coin(&self, coin: &Coin) -> Res;
    fn require_coins(&self, funds: &Coins) -> Res;
    fn defund(&self) -> Res;
}

impl MessageFunds for MessageInfo {
    fn require_coin(&self, expected: &Coin) -> Res {
        if self.funds.len() == 0 {
            return Err(Error::InsufficientFunds {});
        }
        let funds: Coins = self.funds.clone().try_into()?;
        if funds.amount_of(&expected.denom) < expected.amount {
            return Err(Error::InsufficientFunds {});
        }
        Ok(())
    }

    fn require_coins(&self, expected: &Coins) -> Res {
        if self.funds.len() < expected.len() {
            return Err(Error::InsufficientFunds {});
        }
        let funds: Coins = self.funds.clone().try_into()?;
        for coin in expected {
            if funds.amount_of(&coin.denom) < coin.amount {
                return Err(Error::InsufficientFunds {});
            }
        }
        Ok(())
    }

    fn defund(&self) -> Res {
        if self.funds.len() != 0 {
            return Err(Error::FundsNotRequired {});
        }
        Ok(())
    }
}

pub trait AddSplitMessages {
    fn add_split_messages(self, funds: &[Coin], split: &Split) -> Res<Response>;
}

impl AddSplitMessages for Response {
    fn add_split_messages(self, funds: &[Coin], split: &Split) -> Res<Response> {
        let coins: Coins = funds.try_into()?;
        let response = self.add_messages(split.split(&coins)?);
        Ok(response)
    }
}

// ======= MESSAGES =======
#[cw_serde]
pub struct ClaimMsg {
    pub owner: String,
    pub bps: u32,
}

impl Validate<Claim> for ClaimMsg {
    fn validate(&self, deps: Deps) -> Res<Claim> {
        Ok(Claim {
            owner: deps.api.addr_validate(&self.owner)?,
            bps: self.bps,
        })
    }
}

impl From<Claim> for ClaimMsg {
    fn from(claim: Claim) -> ClaimMsg {
        ClaimMsg {
            owner: claim.owner.to_string(),
            bps: claim.bps,
        }
    }
}

#[cw_serde]
pub struct SplitMsg {
    pub claims: Vec<ClaimMsg>,
}

impl Validate<Split> for SplitMsg {
    fn validate(&self, deps: Deps) -> Res<Split> {
        Ok(Split {
            claims: self.claims.validate(deps)?,
        })
    }
}

impl From<Split> for SplitMsg {
    fn from(split: Split) -> SplitMsg {
        SplitMsg {
            claims: split.claims.into_iter().map(Into::into).collect(),
        }
    }
}
