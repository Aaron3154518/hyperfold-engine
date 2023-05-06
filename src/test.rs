use crate::ecs;
use crate::ecs::component;
use crate::ecs::component::Label;
use crate::ecs::entity;
use crate::framework::render_system::{Camera, Elevation, Image, RenderSystem, Screen};
use crate::framework::{event_system, physics};
use crate::includes::*;
use crate::sdl2::SDL_KeyCode::{SDLK_a, SDLK_d, SDLK_s, SDLK_w};
use crate::utils::{event, rect::Align, rect::PointF, rect::Rect};

#[ecs::component]
struct FBallTimer(pub u32);

#[ecs::component(Label)]
struct FBall;

#[ecs::component(Label)]
struct Player;

#[ecs::system(Init)]
fn init_player(entities: &mut crate::CFoo, rs: &mut RenderSystem, screen: &Screen) {
    let e = entity::new();
    entities.add_component(e, Elevation(1));
    let img_w = screen.0.w.min(screen.0.h) / 8;
    entities.add_component(
        e,
        physics::Position(Rect {
            x: (screen.0.w - img_w) as f32 / 2.0,
            y: (screen.0.h - img_w) as f32 / 2.0,
            w: img_w as f32,
            h: img_w as f32,
        }),
    );
    entities.add_component(e, physics::PhysicsData::new());
    entities.add_component(e, Image(rs.get_image("res/bra_vector.png")));
    entities.add_component(e, FBallTimer(1));
    entities.add_label(e, Player);
}

#[ecs::system]
fn on_left_click(ev: &event_system::Events::Mouse, pos: &mut physics::Position, _l: Label<Player>) {
    if ev.0.mouse == event::Mouse::Left && ev.0.clicked() {
        pos.0.set_pos(
            ev.0.click_pos.x as f32,
            ev.0.click_pos.y as f32,
            Align::Center,
            Align::Center,
        );
    }
}

#[ecs::system]
fn key_move(ev: &event_system::Events::Key, pd: &mut physics::PhysicsData, _l: Label<Player>) {
    static V: f32 = 250.0;
    if ev.0.down() {
        match ev.0.key {
            SDLK_a => pd.v.x -= V,
            SDLK_d => pd.v.x += V,
            SDLK_w => pd.v.y -= V,
            SDLK_s => pd.v.y += V,
            _ => (),
        }
    }
    if ev.0.up() {
        match ev.0.key {
            SDLK_a => pd.v.x += V,
            SDLK_d => pd.v.x -= V,
            SDLK_w => pd.v.y += V,
            SDLK_s => pd.v.y -= V,
            _ => (),
        }
    }
}

#[ecs::system]
fn update_player(
    _ev: &ecs::event::CoreEvent::Update,
    pos: &physics::Position,
    camera: &mut Camera,
    _l: Label<Player>,
) {
    camera
        .0
        .set_pos(pos.0.cx(), pos.0.cy(), Align::Center, Align::Center);
}

#[ecs::system]
fn spawn_fb(
    ev: &ecs::event::CoreEvent::Update,
    pos: &mut physics::Position,
    t: &mut FBallTimer,
    entities: &mut crate::CFoo,
    rs: &mut RenderSystem,
    _l: Label<Player>,
) {
    t.0 += ev.0;
    if t.0 >= 1000 {
        t.0 -= 1000;
        let mut r = pos.0.to_owned();
        r.resize(0.5, Align::Center, Align::Center);
        let e = ecs::entity::new();
        entities.add_component(e, physics::Position(r));
        entities.add_component(e, Elevation(0));
        entities.add_component(
            e,
            physics::PhysicsData {
                v: PointF { x: 200.0, y: 200.0 },
                a: PointF::new(),
                boundary: None,
            },
        );
        entities.add_component(e, Image(rs.get_image("res/pump.jpg")));
        entities.add_label(e, FBall);
    }
}

#[ecs::system]
fn update_fb(
    _ev: &ecs::event::CoreEvent::Update,
    pos: &mut physics::Position,
    eid: &ecs::entity::Entity,
    trash: &mut ecs::entity::EntityTrash,
    camera: &Camera,
    _l: Label<FBall>,
) {
    if !pos.0.intersects(&camera.0) {
        trash.0.push(*eid);
    }
}
