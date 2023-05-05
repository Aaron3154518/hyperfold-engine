// Input
#[derive(Debug)]
pub struct Input {
    sm: syn::Ident,
    _1: syn::Token![,],
    cm: syn::Ident,
    _2: syn::Token![,],
    gm: syn::Ident,
    _3: syn::Token![,],
    em: syn::Ident,
}

impl Input {
    pub fn get(self) -> (syn::Ident, syn::Ident, syn::Ident, syn::Ident) {
        (self.sm, self.cm, self.gm, self.em)
    }
}

impl syn::parse::Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sm: input.parse()?,
            _1: input.parse()?,
            cm: input.parse()?,
            _2: input.parse()?,
            gm: input.parse()?,
            _3: input.parse()?,
            em: input.parse()?,
        })
    }
}
