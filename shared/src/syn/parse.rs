use diagnostic::{DiagnosticResult, Error};
use proc_macro2::{TokenStream, TokenTree};
use syn::{parse::ParseStream, spanned::Spanned};

use crate::traits::Increment;

pub trait Parse<T = Self> {
    fn parse(input: ParseStream) -> DiagnosticResult<T>;
}

pub struct DiagnosticResultWrapper<T>(pub DiagnosticResult<T>);

impl<T> syn::parse::Parse for DiagnosticResultWrapper<T>
where
    T: Parse<T>,
{
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let result = T::parse(input);
        // Consume remaining input so that errs are correctly propogated
        // Otherwise syn injects an 'unexpected token' error
        if result.is_err() {
            input.consume();
        }
        Ok(DiagnosticResultWrapper(result))
    }
}

pub trait StreamParse {
    fn parse_stream<T>(self) -> DiagnosticResult<T>
    where
        T: Parse;

    fn consume(self) -> bool;
}

impl StreamParse for ParseStream<'_> {
    fn parse_stream<T>(self) -> DiagnosticResult<T>
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

pub fn parse_tokens<T>(input: TokenStream) -> DiagnosticResult<T>
where
    T: Parse<T>,
{
    let input_span = input.span();
    match syn::parse2::<DiagnosticResultWrapper<T>>(input) {
        Ok(mut t) => {
            // Add input span to empty message spans
            if let Err(msgs) = &mut t.0 {
                msgs.iter_mut().for_each(|msg| {
                    if let Error::Spanned(err) = msg {
                        // *span = span.located_at(input_span);
                    }
                });
            }
            t.0
        }
        Err(e) => Err(vec![Error::spanned(&format!("{e}"), "", &e.span())]),
    }
}
