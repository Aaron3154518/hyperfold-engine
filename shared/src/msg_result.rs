use crate::traits::{PushInto, ThenOk};

// Type for propogating errors
pub type MsgResult<T, E> = Result<T, Vec<E>>;

pub trait MsgTrait<T, E> {
    fn new(t: T, errs: Vec<E>) -> MsgResult<T, E>;

    fn get_ref<'a>(&'a self) -> MsgResult<&'a T, E>;

    // Add rhs errors, don't overwrite data
    fn and_msgs<U>(self, rhs: MsgResult<U, E>) -> MsgResult<T, E>;

    // Add rhs errors, do overwrite data
    fn then_msgs<U>(self, rhs: MsgResult<U, E>) -> MsgResult<U, E>;

    fn add_msg(self, f: impl FnOnce() -> E) -> MsgResult<T, E>;

    fn record_err(self, errs: &mut Vec<E>);

    fn record_err_or(self, errs: &mut Vec<E>, f: impl FnOnce(T));
}

impl<T, E> MsgTrait<T, E> for MsgResult<T, E>
where
    E: Clone,
{
    fn new(t: T, errs: Vec<E>) -> MsgResult<T, E> {
        errs.is_empty().ok(t, errs)
    }

    fn get_ref<'a>(&'a self) -> MsgResult<&'a T, E> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_vec()),
        }
    }

    fn and_msgs<U>(self, rhs: MsgResult<U, E>) -> MsgResult<T, E> {
        match (self, rhs) {
            (Ok(t), Ok(_)) => Ok(t),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(e1), Err(e2)) => Err([e1, e2].concat()),
        }
    }

    fn then_msgs<U>(self, rhs: MsgResult<U, E>) -> MsgResult<U, E> {
        rhs.and_msgs(self)
    }

    fn add_msg(self, f: impl FnOnce() -> E) -> MsgResult<T, E> {
        self.map_err(|errs| errs.push_into(f()))
    }

    fn record_err(self, errs: &mut Vec<E>) {
        match self {
            Ok(_) => (),
            Err(e) => errs.extend(e),
        }
    }

    fn record_err_or(self, errs: &mut Vec<E>, f: impl FnOnce(T)) {
        match self {
            Ok(t) => f(t),
            Err(e) => errs.extend(e),
        }
    }
}

// Combines vectors of messages
pub trait CombineMsgs<T, E> {
    fn combine_msgs(self) -> MsgResult<T, E>;
}

impl<T, E> CombineMsgs<Vec<T>, E> for Vec<MsgResult<T, E>> {
    fn combine_msgs(self) -> MsgResult<Vec<T>, E> {
        let mut msgs = Vec::new();
        let mut ts = Vec::new();
        for msg in self {
            match msg {
                Ok(t) => ts.push(t),
                Err(e) => msgs.extend(e),
            }
        }
        msgs.is_empty().ok(ts, msgs)
    }
}

// Flatten messages
pub trait FlattenMsgs<T, E> {
    fn flatten_msgs(self) -> MsgResult<T, E>;
}

impl<T, E> FlattenMsgs<T, E> for MsgResult<MsgResult<T, E>, E> {
    fn flatten_msgs(self) -> MsgResult<T, E> {
        match self {
            Ok(t) => t,
            Err(e) => Err(e),
        }
    }
}

// Vec<E> -> MsgResult
pub trait ToMsgs<T, E> {
    fn err_or(self, t: T) -> MsgResult<T, E>;
}

impl<T, E> ToMsgs<T, E> for Vec<E> {
    fn err_or(self, t: T) -> MsgResult<T, E> {
        self.is_empty().ok(t, self)
    }
}

macro_rules! msgs_zip {
    ($err: ident, ($tr: ident), ($v0: ident, $vn: ident)) => {
        #[allow(non_snake_case)]
        pub trait $tr<$v0, $vn, $err> {
            fn zip(self, $vn: MsgResult<$vn, $err>) -> MsgResult<($v0, $vn), $err>;
        }

        #[allow(non_snake_case)]
        impl<$v0, $vn, $err> $tr<$v0, $vn, Er> for MsgResult<$v0, $err> where $err: Clone {
            fn zip(self, $vn: MsgResult<$vn, $err>) -> MsgResult<($v0, $vn), $err> {
                match (self, $vn) {
                    (Ok($v0), Ok($vn)) => Ok(($v0, $vn)),
                    (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                    (Err(e1), Err(e2)) => Err([e1, e2].concat()),
                }
            }
        }
    };

    ($err: ident, ($tr: ident, $tr1: ident $(,$trs: ident)*), ($v0: ident, $vn: ident, $vn1: ident $(,$vs: ident)*)) => {
        msgs_zip!($err, ($tr1 $(,$trs)*), ($v0, $vn1 $(,$vs)*));

        #[allow(non_snake_case)]
        pub trait $tr<$v0 $(,$vs)*, $vn1, $vn, $err> {
            fn zip(self $(,$vs: MsgResult<$vs, $err>)*, $vn1: MsgResult<$vn1, $err>, $vn: MsgResult<$vn, $err>)
                -> MsgResult<($v0 $(,$vs)*, $vn1, $vn), $err>;
        }

        #[allow(non_snake_case)]
        impl<$v0 $(,$vs)*, $vn1, $vn, $err> $tr<$v0 $(,$vs)*, $vn1, $vn, $err> for MsgResult<$v0, $err> where $err: Clone {
            fn zip(self $(,$vs: MsgResult<$vs, $err>)*, $vn1: MsgResult<$vn1, $err>, $vn: MsgResult<$vn, $err>)
                -> MsgResult<($v0 $(,$vs)*, $vn1, $vn), $err> {
                    match (<Self as $tr1<$v0 $(,$vs)*, $vn1, $err>>::zip(self $(,$vs)*, $vn1), $vn) {
                        (Ok(($v0 $(,$vs)*, $vn1)), Ok($vn)) => Ok(($v0 $(,$vs)*, $vn1, $vn)),
                        (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                        (Err(e1), Err(e2)) => Err([e1, e2].concat())
                    }
                }
        }
    };
}

msgs_zip!(
    Er,
    (
        Zip26Msgs, Zip25Msgs, Zip24Msgs, Zip23Msgs, Zip22Msgs, Zip21Msgs, Zip20Msgs, Zip19Msgs,
        Zip18Msgs, Zip17Msgs, Zip16Msgs, Zip15Msgs, Zip14Msgs, Zip13Msgs, Zip12Msgs, Zip11Msgs,
        Zip10Msgs, Zip9Msgs, Zip8Msgs, Zip7Msgs, Zip6Msgs, Zip5Msgs, Zip4Msgs, Zip3Msgs, Zip2Msgs
    ),
    (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z)
);

#[macro_export]
macro_rules! match_ok {
    ($v: ident, $ok: block) => {
        $v.map(|$v| $ok)
    };

    ($v: ident, $ok: block, $e: ident, $err: block) => {
        match $v {
            Ok(v) => Ok($ok),
            Err($e) => Err($err),
        }
    };

    ($tr: ident, $v0: ident $(,$vs: ident)*, $ok: block) => {
        $tr::zip($v0 $(,$vs)*).map(|($v0 $(,$vs)*)| $ok)
    };

    ($tr: ident, $v0: ident $(,$vs: ident)*, $ok: block, $e: ident, $err: block) => {
        match $tr::zip($v0 $(,$vs)*) {
            Ok(($v0 $(,$vs)*)) => Ok($ok),
            Err($e) => Err($err)
        }
    };
}
