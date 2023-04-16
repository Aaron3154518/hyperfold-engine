use ecs_lib::component_manager;

use super::component::Component;

// #[component_manager]
pub struct ComponentManager {
    comps: Vec<Component>,
}

impl ComponentManager {
    pub fn add_comp(&mut self, comp: Component) {
        self.comps.push(comp);
    }
}

pub struct ECSDriver {
    comp_man: ComponentManager,
    servs: Vec<Box<dyn Fn(&mut ComponentManager)>>,
}

impl ECSDriver {
    pub fn new() -> Self {
        Self {
            comp_man: ComponentManager { comps: vec![] },
            servs: vec![],
        }
    }

    pub fn add_comp(&mut self, comp: Component) {
        self.comp_man.add_comp(comp);
    }

    pub fn add_serv(&mut self, fun: &'static dyn Fn((&Component,))) {
        self.servs.push(Box::new(|cm: &mut ComponentManager| {
            for c in cm.comps.iter() {
                (fun)((c,))
            }
        }));
    }

    pub fn run(&mut self) {
        for s in self.servs.iter() {
            (s)(&mut self.comp_man)
        }
    }
}
