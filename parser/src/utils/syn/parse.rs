use std::any::type_name;

use proc_macro2::{Span, TokenStream, TokenTree};
use shared::traits::Increment;
use syn::{
    parse::{discouraged::AnyDelimiter, ParseStream},
    spanned::Spanned,
    Error,
};

use crate::utils::{
    msg::{ParseMsg, ParseMsgResult},
    Msg, MsgResult,
};

use super::ToRange;

pub trait Parse<T = Self> {
    fn parse(input: ParseStream) -> ParseMsgResult<T>;
}

pub struct ParseMsgResultWrapper<T>(pub ParseMsgResult<T>);

impl<T> syn::parse::Parse for ParseMsgResultWrapper<T>
where
    T: Parse<T>,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = T::parse(input);
        // Consume remaining input so that errs are correctly propogated
        // Otherwise syn injects an 'unexpected token' error
        if let Err(e) = &result {
            input.consume()
        }
        Ok(ParseMsgResultWrapper(result))
    }
}

pub trait StreamParse {
    fn parse_stream<T>(self) -> ParseMsgResult<T>
    where
        T: Parse;

    fn consume(self);
}

impl StreamParse for ParseStream<'_> {
    fn parse_stream<T>(self) -> ParseMsgResult<T>
    where
        T: Parse,
    {
        T::parse(self)
    }

    fn consume(self) {
        // Parse up to 10000 remaining tokens
        let mut i = 0;
        while self
            .parse::<TokenTree>()
            .is_ok_and(|_| i.add_then(1) < 10000)
        {}
    }
}

pub fn parse_tokens<T>(input: TokenStream) -> ParseMsgResult<T>
where
    T: Parse<T>,
{
    match syn::parse2::<ParseMsgResultWrapper<T>>(input) {
        Ok(t) => t.0,
        Err(e) => Err(vec![ParseMsg::from_span(&format!("{e}"), e.span())]),
    }
}
