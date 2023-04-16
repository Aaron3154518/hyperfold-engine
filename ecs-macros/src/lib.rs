pub trait ComponentManager<T> {
    fn add_component(&mut self, t: T);
}

#[macro_export]
macro_rules! manager {
    ($cm: ident, $($v: ident, $t: ty),*) => {
        pub struct $cm {
            $($v: Vec<$t>),*
        }

        impl $cm {
            pub fn new() -> Self {
                Self {
                    $($v: Vec::new()),*
                }
            }
        }

        $(
            impl ComponentManager<$t> for $cm {
                fn add_component(&mut self, t: $t) {
                    self.$v.push(t)
                }
            }
        )*
    };
}

struct Component;
struct MyComponent;

manager!(Foo, c0, Component, c1, MyComponent);
