use hyperfold_build::env::BuildData;
use regex::Regex;

use super::util::read_data;

#[derive(Clone, Debug)]
pub struct Dependencies {
    pub name: String,
    pub deps: String,
}

impl Dependencies {
    pub fn contains(&self, val: &String) -> bool {
        self.name == *val || self.deps.contains(val)
    }

    pub fn parse() -> Vec<Dependencies> {
        let dep_r =
            Regex::new(r"(?P<name>\w+)\((?P<deps>(\w+(,\w+)*)?)\)").expect("Could not parse regex");
        read_data(BuildData::Dependencies)
            .split(" ")
            .map(|s| {
                let c = dep_r
                    .captures(s)
                    .expect(format!("Could not parse dependencies string: {}", s).as_str());
                Dependencies {
                    name: c
                        .name("name")
                        .expect(format!("Could not parse name for: {}", s).as_str())
                        .as_str()
                        .to_string(),
                    deps: c
                        .name("deps")
                        .expect(format!("Could not parse dependencies for: {}", s).as_str())
                        .as_str()
                        .split(",")
                        .map(|s| s.to_string())
                        .collect(),
                }
            })
            .collect()
    }
}
