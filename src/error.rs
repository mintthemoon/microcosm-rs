use miette::Diagnostic;

#[derive(thiserror::Error, Debug, Diagnostic)]
pub enum Error {
//     #[error(transparent)]
//     Std(#[from] cosmwasm_std::StdError),

//     #[error(transparent)]
//     Cw721(#[from] cw721_base::ContractError),

//     #[error(transparent)]
//     Coins(#[from] cosmwasm_std::CoinsError),

    #[error("Library error: {0}")]
    Library(#[from] Box<dyn std::error::Error>),

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

struct AnyError<T: std::error::Error + 'static>(T);

impl <T: std::error::Error + 'static> From<AnyError<T>> for Error {
    fn from(err: AnyError<T>) -> Self {
        Error::Library(Box::new(err.0))
    }
}

impl From<std::convert::Infallible> for Error {
    fn from(_: std::convert::Infallible) -> Self {
        Error::Unexpected {}
    }
}

impl From<cosmwasm_std::CoinsError> for Error {
    fn from(err: cosmwasm_std::CoinsError) -> Self {
        Error::Library(Box::new(err))
    }
}

impl From<cosmwasm_std::StdError> for Error {
    fn from(err: cosmwasm_std::StdError) -> Self {
        Error::Library(Box::new(err))
    }
}

pub type Res<T = (), E = Error> = Result<T, E>;

pub trait IntoRes<T, E = Error> {
    fn res(self) -> Res<T, E>;
}

impl <T, E: std::error::Error + 'static> IntoRes<T, Error> for Result<T, E> {
    fn res(self) -> Res<T, Error> {
        self.map_err(|e| AnyError(e).into())
    }
}