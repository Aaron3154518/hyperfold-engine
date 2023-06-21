// Parsing macro args
fn parse<T>(input: syn::parse::ParseStream) -> syn::Result<T>
where
    T: for<'a> From<&'a Vec<String>>,
{
    let mut args = Vec::new();
    while let Ok(i) = input.parse::<syn::Ident>() {
        args.push(i.to_string());
        if let Err(e) = input.parse::<syn::Token![,]>() {
            return Err(e);
        }
    }
    Ok(T::from(&args))
}

// Component args
#[derive(Debug, Copy, Clone)]
pub struct ComponentMacroArgs {
    pub is_dummy: bool,
    pub is_singleton: bool,
}

impl From<&Vec<String>> for ComponentMacroArgs {
    fn from(vals: &Vec<String>) -> Self {
        let mut c = Self {
            is_dummy: false,
            is_singleton: false,
        };
        for v in vals {
            match v.as_str() {
                "Dummy" => c.is_dummy = true,
                "Singleton" => c.is_singleton = true,
                "Const" => panic!(
                    "{}\n{}",
                    "Component cannot be Const", "Perhaps you meant to declare this as \"global\"?"
                ),
                _ => panic!("Unknown macro argument for component: {}", v),
            }
        }
        c
    }
}

impl syn::parse::Parse for ComponentMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}

// Global args
#[derive(Debug, Copy, Clone)]
pub struct GlobalMacroArgs {
    pub is_dummy: bool,
    pub is_const: bool,
}

impl From<&Vec<String>> for GlobalMacroArgs {
    fn from(vals: &Vec<String>) -> Self {
        let mut g = Self {
            is_dummy: false,
            is_const: false,
        };
        for v in vals {
            match v.as_str() {
                "Dummy" => g.is_dummy = true,
                "Const" => g.is_const = true,
                _ => panic!("Unknown macro argument for global: {}", v),
            }
        }
        g
    }
}

impl syn::parse::Parse for GlobalMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}

// System args
#[derive(Debug, Copy, Clone)]
pub struct SystemMacroArgs {
    pub is_init: bool,
}

impl From<&Vec<String>> for SystemMacroArgs {
    fn from(vals: &Vec<String>) -> Self {
        let mut s = Self { is_init: false };
        for v in vals {
            match v.as_str() {
                "Init" => s.is_init = true,
                _ => panic!("Unknown macro argument for system: {}", v),
            }
        }
        s
    }
}

impl syn::parse::Parse for SystemMacroArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        parse(input)
    }
}
