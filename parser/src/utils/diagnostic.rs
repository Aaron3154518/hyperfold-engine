use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
        Config,
    },
};

use super::SpanFiles;

pub fn warn(msg: &str) {
    let writer = StandardStream::stderr(ColorChoice::Always);
    term::emit(
        &mut writer.lock(),
        &Config::default(),
        &SpanFiles::new(),
        &Diagnostic::warning().with_message(msg),
    );
}
