pub use macrocosm::cw_error;
pub use crate::std::StdError;

#[cw_error]
pub enum Error {}

pub type Res<T = (), E = Error> = Result<T, E>;

pub trait WrapErr<T> {
    fn wrap_err(inner: T) -> Self;
}

pub trait WrapRes<T, E> {
    fn wrap(self) -> Result<T, E>;
}

impl <T, I, E: WrapErr<I>> WrapRes<T, E> for Result<T, I> {
    fn wrap(self) -> Result<T, E> {
        self.map_err(E::wrap_err)
    }
}

pub trait StdRes<T> {
    #![allow(dead_code)]
    fn std(self) -> Result<T, StdError>;
}

impl <T, E: Into<StdError>> StdRes<T> for Result<T, E> {
    fn std(self) -> Result<T, StdError> {
        self.map_err(Into::into)
    }
}
