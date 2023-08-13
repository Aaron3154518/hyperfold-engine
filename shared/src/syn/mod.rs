// mod msg;
pub mod error;
mod parse;
mod path;
mod quote;
mod span;
mod tokenstream;
mod r#type;
mod uses;
mod util;

pub use self::quote::Quote;
// pub use msg::{
//     CatchErr, CatchSpanErr, DiagnosticResult, GetVec, InjectSpan, Msg, NewError, ParseMsg,
//     DiagnosticResult, SpanFile, ToCompileErr, ToMsg,
// };
pub use parse::{parse_tokens, Parse, StreamParse};
pub use path::{arr_to_path, path_to_vec, string_to_path, vec_to_path};
pub use r#type::{arr_to_type, get_type_generics, string_to_type, type_to_type};
pub use span::ToRange;
pub use tokenstream::format_code;
pub use uses::{add_use_item, use_path_from_syn, use_path_from_vec};
pub use util::{get_fn_name, get_mut};
