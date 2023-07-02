use shared::util::{JoinMap, JoinMapInto, ThenOk};

// Crate index, item index
pub type ItemIndex = (usize, usize);

pub type MsgResult<T> = Result<T, String>;
pub type MsgsResult<T> = Result<T, Vec<String>>;

pub trait ToMsgsResult<T> {
    fn to_msg_vec(self) -> MsgsResult<T>;
}

impl<T> ToMsgsResult<T> for MsgResult<T> {
    fn to_msg_vec(self) -> MsgsResult<T> {
        self.map_err(|e| vec![e])
    }
}

pub trait CombineMsgs<T> {
    fn combine_msgs(self) -> MsgsResult<T>;
}

impl<T> CombineMsgs<Vec<T>> for Vec<MsgResult<T>> {
    fn combine_msgs(self) -> MsgsResult<Vec<T>> {
        let mut msgs = Vec::new();
        let mut ts = Vec::new();
        for msg in self {
            match msg {
                Ok(t) => ts.push(t),
                Err(e) => msgs.push(e),
            }
        }
        msgs.is_empty().result(ts, msgs)
    }
}

impl<T> CombineMsgs<Vec<T>> for Vec<MsgsResult<T>> {
    fn combine_msgs(self) -> MsgsResult<Vec<T>> {
        let mut msgs = Vec::new();
        let mut ts = Vec::new();
        for msg in self {
            match msg {
                Ok(t) => ts.push(t),
                Err(e) => msgs.extend(e),
            }
        }
        msgs.is_empty().result(ts, msgs)
    }
}

macro_rules! msgs_zip {
    (($tr: ident), ($v0: ident, $vn: ident)) => {
        #[allow(non_snake_case)]
        pub trait $tr<$v0, $vn> {
            fn zip(self, $vn: MsgsResult<$vn>) -> MsgsResult<($v0, $vn)>;
        }

        #[allow(non_snake_case)]
        impl<$v0, $vn> $tr<$v0, $vn> for MsgsResult<$v0> {
            fn zip(self, $vn: MsgsResult<$vn>) -> MsgsResult<($v0, $vn)> {
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
            fn zip(self $(,$vs: MsgsResult<$vs>)*, $vn1: MsgsResult<$vn1>, $vn: MsgsResult<$vn>)
                -> MsgsResult<($v0 $(,$vs)*, $vn1, $vn)>;
        }

        #[allow(non_snake_case)]
        impl<$v0 $(,$vs)*, $vn1, $vn> $tr<$v0 $(,$vs)*, $vn1, $vn> for MsgsResult<$v0> {
            fn zip(self $(,$vs: MsgsResult<$vs>)*, $vn1: MsgsResult<$vn1>, $vn: MsgsResult<$vn>)
                -> MsgsResult<($v0 $(,$vs)*, $vn1, $vn)> {
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
    (Zip9Msgs, Zip8Msgs, Zip7Msgs, Zip6Msgs, Zip5Msgs, Zip4Msgs, Zip3Msgs, Zip2Msgs),
    (T, A, B, C, D, E, F, G, H)
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
