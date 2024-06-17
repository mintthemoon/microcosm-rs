use miette::Diagnostic;

#[derive(thiserror::Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    Std(#[from] cosmwasm_std::StdError),

    #[error(transparent)]
    Cw721(#[from] cw721_base::ContractError),

    #[error(transparent)]
    Coins(#[from] cosmwasm_std::CoinsError),

    #[error("User cannot perform this action")]
    Unauthorized {},

    #[error("This function is disabled")]
    Disabled {},

    #[error("Cannot perform this action for this token id")]
    InvalidTokenId {},

    #[error("This token is expired")]
    TokenExpired {},

    #[error("Insufficient funds provided")]
    InsufficientFunds {},

    #[error("This action does not require funds")]
    FundsNotRequired {},
}

pub type Res<T = (), E = Error> = Result<T, E>;