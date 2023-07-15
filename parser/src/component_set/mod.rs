mod codegen;
mod labels;
mod parse;
mod resolve;
pub mod test;

pub use codegen::{BuildSetsArg, BuildSetsResult};
pub use labels::ComponentSetLabels;
pub use resolve::ComponentSet;

#[macro_export]
macro_rules! err {
    ($token: ident, $msg: literal) => {
        Err(Error::new($token.span(), $msg))
    };

    ($token: ident, $msg: literal, $($es: ident),+) => {
        Err({
            let mut err_ = Error::new($token.span(), $msg);
            $(err_.combine($es);)*
            err_
        })
    };
}

#[macro_export]
macro_rules! parse_expect {
    ($tokens: ident, $msg: literal) => {
        match $tokens.parse() {
            Ok(t) => t,
            Err(e) => return err!($tokens, $msg, e),
        }
    };

    ($tokens: ident, $type: ty, $msg: literal) => {
        match $tokens.parse::<$type>() {
            Ok(t) => t,
            Err(e) => return err!($tokens, $msg, e),
        }
    };
}

/*
* Pass 1: Parse into expressions
* Grammar:
* Expr -> Item (Op Item)*
* Item -> !*Ident | !*(Expr)
* Op -> && | ||

* Pass 2: Apply DeMorgan's law
          Split expression sequences with left->right precedence for &&
          Flatten
*/
