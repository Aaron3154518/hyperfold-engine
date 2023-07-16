mod codegen;
mod labels;
mod parse;
mod resolve;
pub mod test;

pub use codegen::{BuildSetsArg, BuildSetsResult};
pub use labels::ComponentSetLabels;
pub use resolve::ComponentSet;

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
