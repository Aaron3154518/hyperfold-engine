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

pub trait ComponentSystems<F> {
    fn add_system(&mut self, f: F);
}

#[macro_export]
macro_rules! zip_tuple {
    // Zip
    ($v1: ident) => {
        $v1
    };

    ($v1: ident, $v2: ident) => {
        ($v2, $v1)
    };

    ($v1: ident $(,$vs: ident)+) => {
        (zip_tuple!($($vs),*), $v1)
    };

    // Reverse
    ((), $($vs: ident),*) => {
        zip_tuple!($($vs),*)
    };

    (($v1: ident $(,$vs: ident)*) $(,$vs2: ident)*) => {
        zip_tuple!(($($vs),*), $v1 $(,$vs2)*)
    };
}

#[macro_export]
macro_rules! systems {
    ($sm: ident, $cm: ident, $(($v1: ident, $t1: ty $(,$vs: ident, $ts: ty)*)),+) => {
        struct $sm {
            pub component_manager: $cm,
            systems: Vec<Box<dyn Fn(&mut $cm)>>,
        }

        impl $sm {
            pub fn new() -> Self {
                Self {
                    component_manager: $cm::new(),
                    systems: Vec::new()
                }
            }

            pub fn tick(&mut self) {
                for system in self.systems.iter() {
                    (system)(&mut self.component_manager);
                }
            }
        }

        $(
            impl ComponentSystems<&'static dyn Fn($t1 $(,$ts)*)> for $sm {
                fn add_system(&mut self, f: &'static dyn Fn($t1 $(,$ts)*)) {
                    self.systems.push(
                        Box::new(|cm: &mut $cm| {
                            for zip_tuple!(($v1 $(,$vs)*)) in cm.$v1.iter_mut()$(.zip(cm.$vs.iter_mut()))* {
                                (f)($v1 $(,$vs)*)
                            }
                        })
                    )
                }
            }
        )*
    };
}

// struct Component;
// struct MyComponent;

// manager!(Foo, c0, Component, c1, MyComponent);
// systems!(
//     SFoo,
//     Foo,
//     (c0, &mut Component, c1, &MyComponent),
//     (c1, &MyComponent)
// );
