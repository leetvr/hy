use std::{collections::VecDeque, time::Instant};

#[derive(Debug, Default, Clone)]
pub struct FrameTimer {
    start_time: Option<Instant>,
    frame_times: VecDeque<f32>,
    last_printed: Option<Instant>,
}

const PRINT_INTERVAL_SECONDS: f32 = 1.;

// How many frame times we should keep to get our rolling average
const MAX_FRAME_TIME_WINDOW: usize = 500;

impl FrameTimer {
    pub fn frame_rate(&self) -> f32 {
        let frame_count = self.frame_times.len();
        let average_time = self.frame_times.iter().sum::<f32>() / frame_count as f32;
        let average_fps = 1.0 / average_time;
        average_fps
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    pub fn stop(&mut self) {
        let Some(start_time) = self.start_time.take() else {
            return;
        };

        let dt = start_time.elapsed().as_secs_f32();
        self.frame_times.push_front(dt);
        self.frame_times.truncate(MAX_FRAME_TIME_WINDOW);

        let last_printed = self.last_printed.get_or_insert_with(Instant::now);
        if last_printed.elapsed().as_secs_f32() < PRINT_INTERVAL_SECONDS {
            return;
        }

        self.print_frame_info();
    }

    fn print_frame_info(&mut self) {
        let frame_count = self.frame_times.len();
        let average_time = self.frame_times.iter().sum::<f32>() / frame_count as f32;
        let average_fps = 1.0 / average_time;
        tracing::debug!(average_time = %average_time, average_fps = %average_fps);
        self.last_printed = Some(Instant::now());
    }
}
