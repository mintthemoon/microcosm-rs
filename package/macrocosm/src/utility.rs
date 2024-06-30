mod unwrap_or_throw {
    macro_rules! unwrap_or_throw {
        ($result:expr) => {
            match $result {
                Ok(value) => value,
                Err(err) => return err.into_compile_error(),
            }
        };
    }

    pub(crate) use unwrap_or_throw;
}

pub(crate) use unwrap_or_throw::unwrap_or_throw;
