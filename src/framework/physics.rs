use crate::{
    components,
    ecs::events::core,
    utils::rect::{PointF, Rect},
};

#[macros::component]
struct Position(pub Rect);

#[derive(Debug)]
#[macros::component]
struct PhysicsData {
    pub v: PointF,
    pub a: PointF,
    pub boundary: Option<Rect>,
}

impl PhysicsData {
    pub fn new() -> Self {
        Self {
            v: PointF::new(),
            a: PointF::new(),
            boundary: None,
        }
    }
}

components!(
    UpdatePhysics,
    pos: &'a mut Position,
    pd: &'a mut PhysicsData,
);

#[macros::system]
fn update_physics(up: &core::Update, entities: Vec<UpdatePhysics>) {
    for UpdatePhysics { pos, pd, .. } in entities {
        let s = up.0 as f32 / 1000.0;
        let a_f = s * s / 2.0;
        pos.0
            .move_by(pd.v.x * s + pd.a.x * a_f, pd.v.y * s + pd.a.y * a_f);
        pd.v.x += pd.a.x * s;
        pd.v.y += pd.a.y * s;
        if let Some(b) = pd.boundary {
            pos.0.move_within(&b);
        }
    }
}

// #[macros::system]
// fn collisions(up: &core::Update, entities: Vec<UpdatePhysics>) {
//     for UpdatePhysics { pos, pd, .. } in entities {
//         let s = up.0 as f32 / 1000.0;
//         let a_f = s * s / 2.0;
//         pos.0
//             .move_by(pd.v.x * s + pd.a.x * a_f, pd.v.y * s + pd.a.y * a_f);
//         pd.v.x += pd.a.x * s;
//         pd.v.y += pd.a.y * s;
//         if let Some(b) = pd.boundary {
//             pos.0.move_within(&b);
//         }
//     }
// }
