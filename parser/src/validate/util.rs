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

pub trait ZipMsgs<T, U> {
    fn zip(self, other: MsgsResult<U>) -> MsgsResult<(T, U)>;
}

impl<T, U> ZipMsgs<T, U> for MsgsResult<T> {
    fn zip(self, other: MsgsResult<U>) -> MsgsResult<(T, U)> {
        match (self, other) {
            (Ok(t), Ok(u)) => Ok((t, u)),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(e_t), Err(e_u)) => Err([e_t, e_u].concat()),
        }
    }
}
