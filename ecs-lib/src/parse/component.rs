use quote::{format_ident, ToTokens};

// Component parser
#[derive(Clone, Debug)]
pub enum ComponentParseType {
    Components,
    Globals,
}

#[derive(Clone, Debug)]
pub struct Component {
    pub var: syn::Ident,
    pub ty: syn::Type,
}

impl Component {
    pub fn find<'a>(components: &'a Vec<Component>, path: &str) -> Option<&'a Component> {
        let path_vec = path.split("::").collect::<Vec<_>>();
        components.iter().find(|s| {
            let mut tts = Vec::new();
            for tt in s.ty.to_token_stream() {
                match tt {
                    proc_macro2::TokenTree::Ident(i) => tts.push(i.to_string()),
                    _ => (),
                }
            }
            tts == path_vec
        })
    }

    pub fn parse(parse_type: ComponentParseType) -> Vec<Self> {
        let (data_key, ty_char) = match parse_type {
            ComponentParseType::Components => ("COMPONENTS", "c"),
            ComponentParseType::Globals => ("GLOBALS", "g"),
        };
        std::env::var(data_key)
            .expect(data_key)
            .split(" ")
            .enumerate()
            .map(|(i, s)| Component {
                var: format_ident!("{}{}", ty_char, i),
                ty: syn::parse_str::<syn::Type>(s)
                    .expect(format!("Could not parse Component type: {:#?}", s).as_str()),
            })
            .collect()
    }
}
