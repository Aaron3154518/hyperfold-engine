pub type Results<T, E> = Result<T, Vec<E>>;

// Convert Vec<U> to Results<T, E>
pub trait ErrForEach<U>
where
    Self: Sized,
{
    fn do_for_each<E>(self, f: impl Fn(U) -> Results<(), E>) -> Vec<E> {
        let (_, errs) = self.try_for_each(f);
        errs
    }

    fn try_for_each<T, E>(self, f: impl Fn(U) -> Results<T, E>) -> (Vec<T>, Vec<E>);
}

impl<U, V> ErrForEach<U> for V
where
    V: IntoIterator<Item = U>,
{
    fn try_for_each<T, E>(self, f: impl Fn(U) -> Results<T, E>) -> (Vec<T>, Vec<E>) {
        let (mut vals, mut errs) = (Vec::new(), Vec::new());
        self.into_iter().for_each(|u| match f(u) {
            Ok(t) => vals.push(t),
            Err(e) => errs.extend(e),
        });
        (vals, errs)
    }
}

// Convert E to Results<T, E>
pub trait ToErr<E> {
    fn vec(self) -> Vec<E>;

    fn err<T>(self) -> Results<T, E>;
}

impl<E> ToErr<E> for E {
    fn vec(self) -> Vec<E> {
        vec![self]
    }

    fn err<T>(self) -> Results<T, E> {
        Err(self.vec())
    }
}

// Convert error type F to error type E
pub trait CatchErr<T, E> {
    fn catch_err(self, err: E) -> Results<T, E>;
}

impl<T, E, F> CatchErr<T, E> for Result<T, F> {
    fn catch_err(self, err: E) -> Results<T, E> {
        self.map_err(|_| err.vec())
    }
}

impl<T, E> CatchErr<T, E> for Option<T> {
    fn catch_err(self, err: E) -> Results<T, E> {
        match self {
            Some(t) => Ok(t),
            None => err.err(),
        }
    }
}

// Convert Vec<E> to Results<T, E>
pub trait ErrorsTrait<E> {
    fn or_else<T>(self, t: T) -> Results<T, E>;

    fn or_then<T>(self, f: impl FnOnce() -> T) -> Results<T, E>;

    fn take_errs<T>(&mut self, r: Results<T, E>) -> Option<T>;
}

impl<E> ErrorsTrait<E> for Vec<E> {
    fn or_else<T>(self, t: T) -> Results<T, E> {
        match self.is_empty() {
            true => Ok(t),
            false => Err(self),
        }
    }

    fn or_then<T>(self, f: impl FnOnce() -> T) -> Results<T, E> {
        match self.is_empty() {
            true => Ok(f()),
            false => Err(self),
        }
    }

    fn take_errs<T>(&mut self, r: Results<T, E>) -> Option<T> {
        match r {
            Ok(t) => Some(t),
            Err(e) => {
                self.extend(e);
                None
            }
        }
    }
}

// Combine multiple ResultsTrait<T, E>
pub trait ResultsTrait<T, E> {
    // Adds/sets errors if rhs is Err
    fn take_errs<U>(self, rhs: Results<U, E>) -> Results<T, E>;

    // Adds errs to vec
    fn record_errs(self, errs: &mut Vec<E>) -> Option<T>;
}

impl<T, E> ResultsTrait<T, E> for Results<T, E> {
    fn take_errs<U>(self, rhs: Results<U, E>) -> Results<T, E> {
        match (self, rhs) {
            (Ok(t), Ok(_)) => Ok(t),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(mut e1), Err(e2)) => {
                e1.extend(e2);
                Err(e1)
            }
        }
    }

    fn record_errs(self, errs: &mut Vec<E>) -> Option<T> {
        match self {
            Ok(t) => Some(t),
            Err(es) => {
                errs.extend(es);
                None
            }
        }
    }
}

// Convert Vec<Results<T, E>> -> Results<Vec<T>, E>
pub trait CombineResults<T, E> {
    fn combine_results(self) -> Results<Vec<T>, E>;
}

impl<T, E> CombineResults<T, E> for Vec<Results<T, E>> {
    fn combine_results(self) -> Results<Vec<T>, E> {
        let mut es = Vec::new();
        let mut ts = Vec::new();
        for msg in self {
            match msg {
                Ok(t) => ts.push(t),
                Err(e) => es.extend(e),
            }
        }
        es.or_else(ts)
    }
}

// Flatten Results<Results<T, E>> -> Results<T, E>
pub trait FlattenResults<T, E> {
    fn flatten_results(self) -> Results<T, E>;
}

impl<T, E> FlattenResults<T, E> for Results<Results<T, E>, E> {
    fn flatten_results(self) -> Results<T, E> {
        self.unwrap_or_else(|e| Err(e))
    }
}

// Zip multiple heterogenous results
pub trait ZipResults<const N: usize, I, O, Er> {
    fn zip(self, i: I) -> Results<O, Er>;
}

macro_rules! zip_results {
    ($r: ident, $tr: ident, $err: ident, ($n: literal), ($a: ident)) => {
        impl<$a, $err> $tr<$n, (), $a, $err> for $r<$a, $err> {
            fn zip(self, _: ()) -> $r<$a, $err> {
                self
            }
        }
    };

    ($r: ident, $tr: ident, $err: ident, ($n1: literal $(,$ns: literal)+), ($a1: ident, $a2: ident $(,$as: ident)*)) => {
        zip_results!($r, $tr, $err, ($($ns),*), ($a2 $(,$as)*));

        #[allow(unused_parens)]
        impl<$a1, $a2 $(,$as)*, $err> $tr<$n1, ($r<$a2, $err> $(,$r<$as, $err>)*), ($a1, $a2 $(,$as)*), $err>
            for $r<$a1, $err>
        {
            #[allow(non_snake_case)]
            fn zip(self, ($a2 $(,$as)*): ($r<$a2, $err> $(,$r<$as, $err>)*)) -> $r<($a1, $a2 $(,$as)*), $err> {
                match (self, $a2.zip(($($as),*))) {
                    (Ok($a1), Ok(($a2 $(,$as)*))) => Ok(($a1, $a2 $(,$as)*)),
                    (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                    (Err(mut e1), Err(e2)) => {
                        e1.extend(e2);
                        Err(e1)
                    }
                }
            }
        }
    };
}

zip_results!(
    Results,
    ZipResults,
    Er,
    (
        26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2,
        1
    ),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)
);

#[macro_export]
macro_rules! zip_match {
    // Add custom base cases to avoid empty parentheses
    (($v: ident) => $ok: block) => {
        $v.map(|$v| $ok)
    };

    (($v: ident) => $ok: block, ($e: ident) => $err: block) => {
        match $v {
            Ok(v) => Ok($ok),
            Err($e) => Err($err),
        }
    };

    (($v0: ident $(,$vs: ident)*) => $ok: block) => {
        $v0.zip(($($vs),*)).map(|($v0 $(,$vs)*)| $ok)
    };

    (($v0: ident $(,$vs: ident)*) => $ok: block, ($e: ident) => $err: block) => {
        match $v0.zip(($($vs),*)) {
            Ok(($v0 $(,$vs)*)) => Ok($ok),
            Err($e) => Err($err)
        }
    };
}