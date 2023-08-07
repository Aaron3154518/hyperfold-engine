use std::{cell::LazyCell, ops::Range};

use proc_macro2::Span;

const SPAN_REGEX: LazyCell<regex::Regex> =
    LazyCell::new(|| regex::Regex::new(r"bytes\((?P<start>\d+)\.\.(?P<end>\d+)\)").unwrap());

pub trait ToRange {
    fn to_range(&self) -> Result<Range<usize>, String>;

    fn range_start(&self) -> Result<usize, String> {
        self.to_range().map(|r| r.start)
    }
}

impl ToRange for Span {
    fn to_range(&self) -> Result<Range<usize>, String> {
        let span = format!("{self:#?}");
        match SPAN_REGEX.captures(span.as_str()) {
            Some(c) => match (c.name("start"), c.name("end")) {
                (Some(s), Some(e)) => {
                    match (s.as_str().parse::<usize>(), e.as_str().parse::<usize>()) {
                        (Ok(s), Ok(e)) => Ok(s..e),
                        (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e.to_string()),
                        (Err(e1), Err(e2)) => Err(format!("{e1}\n{e2}")),
                    }
                }
                _ => Err(format!("Failed to parse span: {span}")),
            },
            None => Err(format!("Failed to parse span: {span}")),
        }
    }
}
