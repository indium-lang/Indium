use crate::{
    ast::Event,
    diagnostic::{Result, Span},
    interpreter::Interpreter,
};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
pub struct FixedStep {
    pub accumulated: f64,
}
impl FixedStep {
    pub fn advance(&mut self, elapsed: f64, tps: f64, max_catchup: usize) -> (usize, u64) {
        self.accumulated += elapsed.max(0.0);
        let step = 1.0 / tps;
        let due = ((self.accumulated / step) + 1e-9).floor().max(0.0) as u64;
        self.accumulated = (self.accumulated - due as f64 * step).max(0.0);
        let ticks = due.min(max_catchup as u64) as usize;
        (ticks, due.saturating_sub(ticks as u64))
    }
}
pub fn run(i: &mut Interpreter) -> Result<()> {
    let graphical = i.mode_2d();
    if !graphical && i.handlers.iter().any(|h| matches!(h.event, Event::Frame)) {
        return Err(i.error("every frame needs mode = 2d", Span::default()));
    }
    if !graphical && i.handlers.is_empty() {
        return Ok(());
    }
    let tick_indices = i
        .handlers
        .iter()
        .enumerate()
        .filter(|(_, h)| matches!(h.event, Event::Tick))
        .map(|(n, _)| n)
        .collect::<Vec<_>>();
    let frame_indices = i
        .handlers
        .iter()
        .enumerate()
        .filter(|(_, h)| matches!(h.event, Event::Frame))
        .map(|(n, _)| n)
        .collect::<Vec<_>>();
    let timer_indices = i
        .handlers
        .iter()
        .enumerate()
        .filter(|(_, h)| matches!(h.event, Event::Timer { .. }))
        .map(|(n, _)| n)
        .collect::<Vec<_>>();
    let has_ticks = !tick_indices.is_empty();
    let mut fixed = FixedStep::default();
    i.started = Instant::now();
    let mut previous = i.started;
    let mut last_frame = i.started;
    let mut next_frame = i.started;
    while !i.stopped {
        if i.options
            .max_frames
            .is_some_and(|n| i.frame_count as u64 >= n)
        {
            break;
        }
        if graphical {
            let quit = i
                .media
                .poll_events()
                .map_err(|e| i.error(e, Span::default()))?;
            if quit {
                break;
            }
        }
        let now = Instant::now();
        let elapsed = (now - previous).as_secs_f64();
        previous = now;
        if has_ticks {
            let (ticks, dropped) = fixed.advance(elapsed, i.tps, i.max_catchup);
            i.dropped_ticks = i.dropped_ticks.saturating_add(dropped);
            for _ in 0..ticks {
                if i.stopped {
                    break;
                }
                i.tick_count = i.tick_count.saturating_add(1);
                i.tick_dt = 1.0 / i.tps;
                i.media.tick_phase();
                for index in &tick_indices {
                    if !i.stopped {
                        i.dispatch(*index)?;
                    }
                }
                i.media.end_tick();
            }
        }
        let elapsed = (now - i.started).as_secs_f64();
        for index in &timer_indices {
            if !i.stopped && elapsed + 1e-9 >= i.handlers[*index].next {
                let h = &mut i.handlers[*index];
                let missed = ((elapsed - h.next) / h.interval).floor().max(0.0) + 1.0;
                h.next += missed * h.interval;
                i.dispatch(*index)?;
            }
        }
        if graphical && !i.stopped && now >= next_frame {
            i.frame_dt = (now - last_frame).as_secs_f64();
            last_frame = now;
            i.media.frame_phase();
            for index in &frame_indices {
                if !i.stopped {
                    i.dispatch(*index)?;
                }
            }
            if !i.stopped {
                i.media.present();
            }
            i.media.end_frame();
            i.frame_count = i.frame_count.saturating_add(1);
            next_frame = now
                + Duration::from_secs_f64(if i.media.config.fps == 0 {
                    0.0
                } else {
                    1.0 / i.media.config.fps as f64
                });
        }
        if i.stopped {
            break;
        }
        let mut wait = 0.01f64;
        if has_ticks {
            wait = wait.min((1.0 / i.tps - fixed.accumulated).max(0.0));
        }
        if graphical {
            wait = wait.min(
                next_frame
                    .saturating_duration_since(Instant::now())
                    .as_secs_f64(),
            );
        }
        for index in &timer_indices {
            wait = wait.min((i.handlers[*index].next - i.started.elapsed().as_secs_f64()).max(0.0));
        }
        if wait > 0.0001 {
            thread::sleep(Duration::from_secs_f64(wait));
        }
    }
    Ok(())
}
