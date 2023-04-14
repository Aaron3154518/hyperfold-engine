use uuid::Uuid;

struct Entity(Uuid);

impl Entity {
    pub fn new() -> Self {
        Self { 0: Uuid::new_v4() }
    }
}
