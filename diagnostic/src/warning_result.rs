use super::{CriticalResult, Result};

// Used for operations that have non-critical errors (still produces a value)
pub struct WarningResult<T, E> {
    pub value: T,
    pub errors: Vec<E>,
}

impl<T, E> WarningResult<T, E> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> WarningResult<U, E> {
        WarningResult {
            value: f(self.value),
            errors: self.errors,
        }
    }

    pub fn try_map<U>(self, f: impl FnOnce(T) -> CriticalResult<U, E>) -> Result<U, E> {
        self.map(f).extract_error()
    }

    pub fn record_errs(self, errs: &mut Vec<E>) -> T {
        errs.extend(self.errors);
        self.value
    }
}

impl<T, E> WarningResult<CriticalResult<T, E>, E> {
    pub fn extract_error(self) -> Result<T, E> {
        self.value.map(|value| WarningResult {
            value,
            errors: self.errors,
        })
    }
}

impl<T, E> From<(T, Vec<E>)> for WarningResult<T, E> {
    fn from((value, errors): (T, Vec<E>)) -> Self {
        Self { value, errors }
    }
}

pub fn ok<T, E>(value: T) -> WarningResult<T, E> {
    err(value, Vec::new())
}

pub fn err<T, E>(value: T, errors: Vec<E>) -> WarningResult<T, E> {
    WarningResult { value, errors }
}

// Convert Vec<WarningResult<T, E>> to WarningResult<Vec<T>, E>
pub trait CombineWarnings<T, E> {
    fn combine_results(self) -> WarningResult<Vec<T>, E>;
}

impl<T, E> CombineWarnings<T, E> for Vec<WarningResult<T, E>> {
    fn combine_results(self) -> WarningResult<Vec<T>, E> {
        let mut errors = Vec::new();
        WarningResult {
            value: self
                .into_iter()
                .map(|r| {
                    errors.extend(r.errors);
                    r.value
                })
                .collect(),
            errors,
        }
    }
}

pub trait Swap<T, U> {
    fn swap(self) -> (U, T);
}

impl<T, U> Swap<T, U> for (T, U) {
    fn swap(self) -> (U, T) {
        (self.1, self.0)
    }
}

// Convert Vec<U> to Results<T, E>
pub trait ErrForEach<U>
where
    Self: Sized,
{
    fn try_for_each<T, E>(
        self,
        f: impl FnMut(U) -> CriticalResult<T, E>,
    ) -> WarningResult<Vec<T>, E>;

    fn try_filter_for_each<T, E>(
        self,
        f: impl FnMut(U) -> CriticalResult<Option<T>, E>,
    ) -> WarningResult<Vec<T>, E>;

    fn try_until<T, E>(
        self,
        f: impl FnMut(U) -> CriticalResult<Option<T>, E>,
    ) -> WarningResult<Option<T>, E>;
}

impl<U, V> ErrForEach<U> for V
where
    V: IntoIterator<Item = U>,
{
    fn try_for_each<T, E>(
        self,
        mut f: impl FnMut(U) -> CriticalResult<T, E>,
    ) -> WarningResult<Vec<T>, E> {
        let (mut vals, mut errs) = (Vec::new(), Vec::new());
        self.into_iter().for_each(|u| match f(u) {
            Ok(t) => vals.push(t),
            Err(e) => errs.extend(e),
        });
        (vals, errs).into()
    }

    fn try_filter_for_each<T, E>(
        self,
        mut f: impl FnMut(U) -> CriticalResult<Option<T>, E>,
    ) -> WarningResult<Vec<T>, E> {
        let (mut vals, mut errs) = (Vec::new(), Vec::new());
        self.into_iter().for_each(|u| match f(u) {
            Ok(t) => {
                if let Some(t) = t {
                    vals.push(t);
                }
            }
            Err(e) => errs.extend(e),
        });
        (vals, errs).into()
    }

    fn try_until<T, E>(
        self,
        mut f: impl FnMut(U) -> CriticalResult<Option<T>, E>,
    ) -> WarningResult<Option<T>, E> {
        let mut errs = Vec::new();
        (
            self.into_iter().find_map(|u| match f(u) {
                Ok(t) => t,
                Err(e) => {
                    errs.extend(e);
                    None
                }
            }),
            errs,
        )
            .into()
    }
}
