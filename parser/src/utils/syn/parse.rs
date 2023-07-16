use std::any::type_name;

use proc_macro2::{Span, TokenStream, TokenTree};
use shared::traits::{Increment, RangeTrait};
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
            input.consume();
        }
        Ok(ParseMsgResultWrapper(result))
    }
}

pub trait StreamParse {
    fn parse_stream<T>(self) -> ParseMsgResult<T>
    where
        T: Parse;

    fn consume(self) -> bool;
}

impl StreamParse for ParseStream<'_> {
    fn parse_stream<T>(self) -> ParseMsgResult<T>
    where
        T: Parse,
    {
        T::parse(self)
    }

    // Consumes up to 10000 remaining tokens, returns true if tokens were consumed
    fn consume(self) -> bool {
        let mut i = 0;
        while self
            .parse::<TokenTree>()
            .is_ok_and(|_| i.add_then(1) < 10000)
        {}

        i != 0
    }
}

pub fn parse_tokens<T>(input: TokenStream) -> ParseMsgResult<T>
where
    T: Parse<T>,
{
    let span = input.span();
    match syn::parse2::<ParseMsgResultWrapper<T>>(input) {
        Ok(mut t) => {
            // Add input span to empty message spans
            if let (Err(msgs), Ok(start)) = (&mut t.0, span.range_start()) {
                msgs.iter_mut().for_each(|msg| {
                    if let ParseMsg::Diagnostic { span, .. } = msg {
                        if span.start == 0 && span.end == 0 {
                            span.add(start);
                        }
                    }
                });
            }
            t.0
        }
        Err(e) => Err(vec![ParseMsg::from_span(&format!("{e}"), e.span())]),
    }
}
