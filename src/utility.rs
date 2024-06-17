use cosmwasm_std::{Addr, Deps};
use crate::{Res, Error};

pub trait Authorize {
    fn authorize(&self, addr: &Addr) -> Res;
}

impl Authorize for Addr {
    fn authorize(&self, addr: &Addr) -> Res {
        if self != addr {
            return Err(Error::Unauthorized {});
        }
        Ok(())
    }
}

pub trait Validate<T> {
    fn validate(&self, deps: Deps) -> Res<T>;
}

impl <T, U> Validate<Vec<U>> for Vec<T> where T: Validate<U> {
    fn validate(&self, deps: Deps) -> Res<Vec<U>> {
        self.into_iter().map(|x| x.validate(deps)).collect()
    }
}

impl <T, U> Validate<Option<U>> for Option<T> where T: Validate<U> {
    fn validate(&self, deps: Deps) -> Res<Option<U>> {
        match self {
            Some(x) => x.validate(deps).map(Some),
            None => Ok(None),
        }
    }
}

impl Validate<Addr> for String {
    fn validate(&self, deps: Deps) -> Res<Addr> {
        deps.api.addr_validate(self).map_err(Into::into)
    }
}
