use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use shared::util::{JoinMap, JoinMapInto, PushInto, ThenOk};

// Returns `mut` or ``
pub fn get_mut(is_mut: bool) -> TokenStream {
    match is_mut {
        true => quote!(mut),
        false => quote!(),
    }
}

// Returns `{ident}` if not mut, otherwise `{ident}_mut`
pub fn get_fn_name(ident: &str, is_mut: bool) -> syn::Ident {
    match is_mut {
        true => format_ident!("{ident}_mut"),
        false => format_ident!("{ident}"),
    }
}

// Crate index, item index
pub type ItemIndex = (usize, usize);

// Type for propogating errors
pub type MsgsResult<T> = Result<T, Vec<String>>;

// Traits for both msg types
pub trait MsgTrait<T> {
    fn get_ref<'a>(&'a self) -> MsgsResult<&'a T>;

    // Add rhs errors, don't overwrite data
    fn and_msgs<U>(self, rhs: MsgsResult<U>) -> MsgsResult<T>;

    // Add rhs errors, do overwrite data
    fn then_msgs<U>(self, rhs: MsgsResult<U>) -> MsgsResult<U>;
}

impl<T> MsgTrait<T> for MsgsResult<T> {
    fn get_ref<'a>(&'a self) -> MsgsResult<&'a T> {
        match self {
            Ok(t) => Ok(t),
            Err(e) => Err(e.to_vec()),
        }
    }

    fn and_msgs<U>(self, rhs: MsgsResult<U>) -> MsgsResult<T> {
        match (self, rhs) {
            (Ok(t), Ok(_)) => Ok(t),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(e1), Err(e2)) => Err([e1, e2].concat()),
        }
    }

    fn then_msgs<U>(self, rhs: MsgsResult<U>) -> MsgsResult<U> {
        rhs.and_msgs(self)
    }
}

// Combines vectors of messages
pub trait CombineMsgs<T> {
    fn combine_msgs(self) -> MsgsResult<T>;
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
        msgs.is_empty().ok(ts, msgs)
    }
}

// Flatten messages
pub trait FlattenMsgs<T> {
    fn flatten_msgs(self) -> MsgsResult<T>;
}

impl<T> FlattenMsgs<T> for MsgsResult<MsgsResult<T>> {
    fn flatten_msgs(self) -> MsgsResult<T> {
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
