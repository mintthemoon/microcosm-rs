use cosmwasm_std::{CoinsError, StdError};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Internal error: {msg:?}")]
    Generic { msg: String },

    #[error(transparent)]
    Std(#[from] StdError),

    #[error("User cannot perform this action")]
    Unauthorized {},

    #[error("This function is disabled")]
    Disabled {},

    #[error("Insufficient funds provided")]
    InsufficientFunds {},

    #[error("This action does not require funds")]
    FundsNotRequired {},

    #[error("Provided input was invalid")]
    Input {},

    #[error("Unexpected error occurred")]
    Unexpected {},
}

impl From<CoinsError> for Error {
    fn from(err: CoinsError) -> Error {
        Error::Std(err.into())
    }
}

pub type Res<T = (), E = Error> = Result<T, E>;

pub trait ToRes<T, E = Error> {
    fn res(self) -> Res<T, E>;
}

impl <T, E: Into<anyhow::Error>> ToRes<T> for Result<T, E> {
    fn res(self) -> Res<T> {
        self.map_err(|e| Error::Generic { msg: e.into().to_string() })
    }
}
