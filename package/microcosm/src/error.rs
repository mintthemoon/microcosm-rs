pub use macrocosm::cw_error;
pub use crate::std::StdError;

#[cw_error]
pub enum Error {}

pub trait WrapErr<T> {
    fn wrap_err(inner: T) -> Self;
}

pub trait FromStdError {
    fn from_std<E: Into<StdError>>(err: E) -> Self;
}

pub type Res<T = (), E = Error> = Result<T, E>;

pub trait WrapRes<T, E> {
    fn wrap(self) -> Res<T, E>;
}

impl <T, I, E: WrapErr<I>> WrapRes<T, E> for Result<T, I> {
    fn wrap(self) -> Res<T, E> {
        self.map_err(E::wrap_err)
    }
}

pub trait StdRes<T> {
    fn std(self) -> Res<T, StdError>;
}

impl <T, E: Into<StdError>> StdRes<T> for Res<T, E> {
    fn std(self) -> Res<T, StdError> {
        self.map_err(Into::into)
    }
}
