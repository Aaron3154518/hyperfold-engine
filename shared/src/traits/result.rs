// Trait for getting the value of a Result regardless of Ok/Err
pub trait GetResult<T> {
    fn get(self) -> T;
}

impl<T> GetResult<T> for Result<T, T> {
    fn get(self) -> T {
        match self {
            Ok(t) | Err(t) => t,
        }
    }
}

// then_some for Result
pub trait ThenOk<T, E> {
    fn ok(&self, t: T, e: E) -> Result<T, E>;

    fn then_ok<Ft, Fe>(&self, t: Ft, e: Fe) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E;

    fn err(&self, e: E, t: T) -> Result<T, E>;

    fn then_err<Ft, Fe>(&self, e: Fe, t: Ft) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E;
}

impl<T, E> ThenOk<T, E> for bool {
    fn ok(&self, t: T, e: E) -> Result<T, E> {
        match self {
            true => Ok(t),
            false => Err(e),
        }
    }

    fn then_ok<Ft, Fe>(&self, t: Ft, e: Fe) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E,
    {
        match self {
            true => Ok(t()),
            false => Err(e()),
        }
    }

    fn err(&self, e: E, t: T) -> Result<T, E> {
        match self {
            true => Err(e),
            false => Ok(t),
        }
    }

    fn then_err<Ft, Fe>(&self, e: Fe, t: Ft) -> Result<T, E>
    where
        Ft: FnOnce() -> T,
        Fe: FnOnce() -> E,
    {
        match self {
            true => Err(e()),
            false => Ok(t()),
        }
    }
}

// ok() for Result but handle the error
pub trait HandleErr<T, E> {
    fn handle_err<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(E);
}

impl<T, E> HandleErr<T, E> for Result<T, E> {
    fn handle_err<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(E),
    {
        match self {
            Ok(t) => Some(t),
            Err(e) => {
                f(e);
                None
            }
        }
    }
}
