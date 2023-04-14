use super::component::Component;

pub fn greet(comp: &Component) {
    println!("Hi {} from {}", comp.name, comp.loc)
}
