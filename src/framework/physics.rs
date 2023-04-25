use ecs_lib;

use crate::{
    ecs::event::CoreEvent,
    utils::rect::{PointF, Rect},
};

// TODO type Position = Rect
#[ecs_lib::component]
struct Position(pub Rect);

impl Position {
    pub fn new() -> Self {
        Self(Rect::new())
    }
}

#[ecs_lib::component]
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

// TODO: Include own use path
#[ecs_lib::system]
fn update_physics(
    up: &CoreEvent::Update,
    pos: &mut super::physics::Position,
    pd: &mut super::physics::PhysicsData,
) {
    let s = up.0 as f32 / 1000.0;
    let a_f = s * s / 2.0;
    // println!("{:#?}", pd.v);
    pos.0
        .move_by(pd.v.x * s + pd.a.x * a_f, pd.v.y * s + pd.a.y * a_f);
    pd.v.x += pd.a.x * s;
    pd.v.y += pd.a.y * s;
    if !pd.boundary.empty() {
        pos.0.fit_within(&pd.boundary);
    }
}
