use crate::traits::ThenOk;

// Type for propogating errors
pub type MsgResult<T> = Result<T, Vec<String>>;

pub trait MsgTrait<T> {
    fn new(t: T, errs: Vec<String>) -> MsgResult<T>;

    fn get_ref<'a>(&'a self) -> MsgResult<&'a T>;

    // Add rhs errors, don't overwrite data
    fn and_msgs<U>(self, rhs: MsgResult<U>) -> MsgResult<T>;

    // Add rhs errors, do overwrite data
    fn then_msgs<U>(self, rhs: MsgResult<U>) -> MsgResult<U>;
}

impl<T> MsgTrait<T> for MsgResult<T> {
    fn new(t: T, errs: Vec<String>) -> MsgResult<T> {
        errs.is_empty().ok(t, errs)
    }

    fn get_ref<'a>(&'a self) -> MsgResult<&'a T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_vec()),
        }
    }

    fn and_msgs<U>(self, rhs: MsgResult<U>) -> MsgResult<T> {
        match (self, rhs) {
            (Ok(t), Ok(_)) => Ok(t),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(e1), Err(e2)) => Err([e1, e2].concat()),
        }
    }

    fn then_msgs<U>(self, rhs: MsgResult<U>) -> MsgResult<U> {
        rhs.and_msgs(self)
    }
}

// Combines vectors of messages
pub trait CombineMsgs<T> {
    fn combine_msgs(self) -> MsgResult<T>;
}

impl<T> CombineMsgs<Vec<T>> for Vec<MsgResult<T>> {
    fn combine_msgs(self) -> MsgResult<Vec<T>> {
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
pub trait FlattenMsgs<T> {
    fn flatten_msgs(self) -> MsgResult<T>;
}

impl<T> FlattenMsgs<T> for MsgResult<MsgResult<T>> {
    fn flatten_msgs(self) -> MsgResult<T> {
        match self {
            Ok(t) => t,
            Err(e) => Err(e),
        }
    }
}

macro_rules! msgs_zip {
    (($tr: ident), ($v0: ident, $vn: ident)) => {
        #[allow(non_snake_case)]
        pub trait $tr<$v0, $vn> {
            fn zip(self, $vn: MsgResult<$vn>) -> MsgResult<($v0, $vn)>;
        }

        #[allow(non_snake_case)]
        impl<$v0, $vn> $tr<$v0, $vn> for MsgResult<$v0> {
            fn zip(self, $vn: MsgResult<$vn>) -> MsgResult<($v0, $vn)> {
                match (self, $vn) {
                    (Ok($v0), Ok($vn)) => Ok(($v0, $vn)),
                    (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                    (Err(e1), Err(e2)) => Err([e1, e2].concat()),
                }
            }
        }
    };

    (($tr: ident, $tr1: ident $(,$trs: ident)*), ($v0: ident, $vn: ident, $vn1: ident $(,$vs: ident)*)) => {
        msgs_zip!(($tr1 $(,$trs)*), ($v0, $vn1 $(,$vs)*));

        #[allow(non_snake_case)]
        pub trait $tr<$v0 $(,$vs)*, $vn1, $vn> {
            fn zip(self $(,$vs: MsgResult<$vs>)*, $vn1: MsgResult<$vn1>, $vn: MsgResult<$vn>)
                -> MsgResult<($v0 $(,$vs)*, $vn1, $vn)>;
        }

        #[allow(non_snake_case)]
        impl<$v0 $(,$vs)*, $vn1, $vn> $tr<$v0 $(,$vs)*, $vn1, $vn> for MsgResult<$v0> {
            fn zip(self $(,$vs: MsgResult<$vs>)*, $vn1: MsgResult<$vn1>, $vn: MsgResult<$vn>)
                -> MsgResult<($v0 $(,$vs)*, $vn1, $vn)> {
                    match (<Self as $tr1<$v0 $(,$vs)*, $vn1>>::zip(self $(,$vs)*, $vn1), $vn) {
                        (Ok(($v0 $(,$vs)*, $vn1)), Ok($vn)) => Ok(($v0 $(,$vs)*, $vn1, $vn)),
                        (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
                        (Err(e1), Err(e2)) => Err([e1, e2].concat())
                    }
                }
        }
    };
}

msgs_zip!(
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
