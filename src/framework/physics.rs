use crate::ecs;

use crate::{
    ecs::event::CoreEvent,
    utils::rect::{PointF, Rect},
};

#[ecs::component]
struct Position(pub Rect);

#[ecs::component]
struct PhysicsData {
    pub v: PointF,
    pub a: PointF,
    pub boundary: Rect,
}

impl PhysicsData {
    pub fn new() -> Self {
        Self {
            v: PointF::new(),
            a: PointF::new(),
            boundary: Rect::new(),
        }
    }
}

#[ecs::system]
fn update_physics(up: &CoreEvent::Update, pos: &mut Position, pd: &mut PhysicsData) {
    let s = up.0 as f32 / 1000.0;
    let a_f = s * s / 2.0;
    pos.0
        .move_by(pd.v.x * s + pd.a.x * a_f, pd.v.y * s + pd.a.y * a_f);
    pd.v.x += pd.a.x * s;
    pd.v.y += pd.a.y * s;
    if !pd.boundary.empty() {
        pos.0.fit_within(&pd.boundary);
    }
}
