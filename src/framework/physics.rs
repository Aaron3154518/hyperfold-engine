use crate::{
    _engine::AddEvent,
    components,
    ecs::{entities::Entity, events::core},
    utils::rect::{Align, PointF, Rect},
};

#[macros::component]
struct Position(pub Rect);

#[macros::component]
struct HitBox(pub Rect);

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

#[macros::event]
struct BoundaryCollision(pub Entity);

components!(
    UpdatePhysics,
    pos: &'a mut Position,
    hit_box: Option<&'a mut HitBox>,
    pd: &'a mut PhysicsData,
);

#[macros::system]
fn update_physics(up: &core::Update, entities: Vec<UpdatePhysics>, events: &mut dyn AddEvent) {
    for UpdatePhysics {
        eid,
        pos,
        hit_box,
        pd,
    } in entities
    {
        let s = up.0 as f32 / 1000.0;
        let a_f = s * s / 2.0;
        pos.0
            .move_by(pd.v.x * s + pd.a.x * a_f, pd.v.y * s + pd.a.y * a_f);
        pd.v.x += pd.a.x * s;
        pd.v.y += pd.a.y * s;
        if let Some(b) = pd.boundary {
            if pos.0.move_within(&b) {
                events.new_event(BoundaryCollision(*eid));
            }
        }
        if let Some(hit_box) = hit_box {
            hit_box.0.copy_rect_pos(pos.0, Align::Center, Align::Center);
        }
    }
}
