pub use macrocosm::cw_error;
pub use crate::std::StdError;

#[cw_error]
pub enum Error {}

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
        self.map_err(Into::into)
    }
}
