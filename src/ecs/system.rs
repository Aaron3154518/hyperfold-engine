use super::component::Component;

pub fn greet(comp: (&Component,)) {
    println!("Hi {} from {}", comp.0.name, comp.0.loc)
}
