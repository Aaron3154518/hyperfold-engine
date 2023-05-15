pub const NAMESPACE: &str = "_engine";
pub const SEP: &str = ";";
pub const DATA_FILE: &str = "hyperfold_data.txt";
pub const EID: &str = "id";

pub fn component_var(cr_idx: usize, c_idx: usize) -> String {
    format!("c{}_{}", cr_idx, c_idx)
}

pub fn global_var(cr_idx: usize, g_idx: usize) -> String {
    format!("g{}_{}", cr_idx, g_idx)
}

pub fn event_var(cr_idx: usize, e_idx: usize) -> String {
    format!("e{}_{}", cr_idx, e_idx)
}

pub fn event_variant(cr_idx: usize, e_idx: usize) -> String {
    format!("E{}_{}", cr_idx, e_idx)
}
