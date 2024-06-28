use crate::std::StdError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Internal error: {0}")]
    Generic(String),

    #[error(transparent)]
    Std(StdError),

    #[error("User cannot perform this action")]
    Unauthorized,

    #[error("This function is disabled")]
    Disabled,

    #[error("This {0} has expired")]
    Expired(&'static str),

    #[error("Insufficient funds provided")]
    InsufficientFunds,

    #[error("This action does not require funds")]
    FundsNotRequired,

    #[error("Provided input was invalid")]
    Input,

    #[error("Unexpected error occurred")]
    Unexpected,
}

impl <T: Into<StdError>> From<T> for Error {
    fn from(err: T) -> Error {
        Error::Std(err.into())
    }
}

impl Error {
    #![allow(dead_code)]
    fn std(self) -> StdError {
        match self {
            Error::Std(err) => err,
            _ => StdError::GenericErr { msg: self.to_string() },
        }
    }
}

pub type Res<T = (), E = Error> = Result<T, E>;

pub trait ToRes<T, E = Error> {
    fn res(self) -> Res<T, E>;
}

impl <T, E: Into<anyhow::Error>> ToRes<T> for Result<T, E> {
    fn res(self) -> Res<T> {
        self.map_err(|e| Error::Generic(e.into().to_string()))
    }
}

pub trait FromRes<T> {
    fn std(self) -> Res<T, StdError>;
}

impl <T> FromRes<T> for Res<T> {
    fn std(self) -> Res<T, StdError> {
        self.map_err(Error::std)
    }
}
