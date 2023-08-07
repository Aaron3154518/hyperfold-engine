use codespan_reporting::term::termcolor::Ansi;

pub type Writer = Ansi<Vec<u8>>;

pub trait WriterTrait {
    fn empty() -> Self;

    fn to_string(&self) -> String;
}

impl WriterTrait for Writer {
    fn empty() -> Self {
        Self::new(Vec::new())
    }

    fn to_string(&self) -> String {
        std::str::from_utf8(self.get_ref().as_slice())
            .unwrap()
            .to_string()
    }
}
