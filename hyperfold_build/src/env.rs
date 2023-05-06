// Environment variables to use
pub const DATA_FILE: &str = "hyperfold_build_data.txt";
#[derive(Clone, Copy, Debug)]
pub enum BuildData {
    Components,
    Globals,
    Events,
    Systems,
    Dependencies,
}
