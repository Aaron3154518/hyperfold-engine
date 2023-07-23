// Trait that can be implemented for Timer wrappers
pub trait TimerTrait {
    fn new(length: u32) -> Self;

    fn get_timer(&self) -> &Timer;
    fn get_timer_mut(&mut self) -> &mut Timer;

    fn add_time(&mut self, dt: u32) -> u32 {
        let timer = self.get_timer_mut();
        match timer.length {
            0 => 1,
            length => {
                timer.time += dt;
                let n = timer.time / length;
                timer.time %= length;
                n
            }
        }
    }

    fn time_passed(&self) -> u32 {
        self.get_timer().time
    }

    fn time_left(&self) -> u32 {
        let t = self.get_timer();
        t.length - t.time
    }

    fn progress(&self) -> f32 {
        let t = self.get_timer();
        t.time as f32 / t.length as f32
    }
}

#[macros::component]
struct Timer {
    pub length: u32,
    time: u32,
}

impl TimerTrait for Timer {
    fn new(length: u32) -> Self {
        Self { length, time: 0 }
    }

    fn get_timer(&self) -> &Timer {
        self
    }

    fn get_timer_mut(&mut self) -> &mut Timer {
        self
    }
}

// TODO: Target events at specific entities
// use crate::_engine::AddEvent;
// use hyperfold_engine::ecs::{entities::Entity, events::core::Update};

// #[hyperfold_engine::event]
// struct TimerFinished(pub Entity);

// hyperfold_engine::components!(UpdateTimers, timer: &'a mut Timer);

// #[hyperfold_engine::system]
// fn update_timers(update: &Update, timers: Vec<UpdateTimers>, events: &dyn AddEvent) {
//     for UpdateTimers {
//         eid,
//         timer: Timer { length, time },
//     } in timers
//     {
//         match length {
//             0 => events.new_event(TimerFinished(*eid)),
//             length => {
//                 *time += update.0;
//                 while time >= length {
//                     *time -= *length;
//                     events.new_event(TimerFinished(*eid));
//                 }
//             }
//         }
//     }
// }
