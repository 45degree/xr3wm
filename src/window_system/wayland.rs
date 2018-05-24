use errors::*;

use std::thread;
use slog::Logger;
#[macro_use]
use slog_scope;
use slog::DrainExt;
use rustwlc as wlc;
use self::wlc::{WlcView, callback, WlcOutput};
use self::wlc::types::*;
use window_system::*;

use std::cmp;

fn log_callback(log_type: wlc::LogType, text: &str) {
    match log_type {
        wlc::LogType::Info => info!("{}", text),
        wlc::LogType::Warn => warn!("{}", text),
        wlc::LogType::Error => error!("{}", text),
        wlc::LogType::Wayland => debug!("{}", text)
    }
}

fn render_output(output: WlcOutput) {
    let resolution = output.get_resolution().unwrap();
    let views = output.get_views();
    if views.is_empty() { return; }

    let mut toggle = false;
    let mut y = 0;
    let w = resolution.w / 2;
    let h = resolution.h / cmp::max((views.len() + 1) / 2, 1) as u32;
    for (i, view) in views.iter().enumerate() {
        view.set_geometry(ResizeEdge::empty(), Geometry {
            origin: Point { x: if !toggle { w as i32 } else { 0 }, y: y },
            size: Size { w: if !toggle && i == views.len() - 1 { resolution.w } else { w }, h: h }
        });
        y += if toggle { h as i32 } else { 0 };
        toggle ^= true;
    }
}

extern fn on_output_resolution(output: WlcOutput, from: &Size, to: &Size) {
    render_output(output);
}

extern fn on_view_created(view: WlcView) -> bool {
    view.set_mask(view.get_output().get_mask());
    view.bring_to_front();
    view.focus();
    render_output(view.get_output());
    true
}

extern fn on_view_focus(view: WlcView, focused: bool) {
    view.set_state(VIEW_ACTIVATED, focused);
}

pub struct Wayland {
    log: Logger,
    run: fn()
}

impl WindowSystem for Wayland {
    fn initialize() -> Result<Wayland> {
        info!("initializing Wayland backend");

        callback::output_resolution(on_output_resolution);
        callback::view_created(on_view_created);
        callback::view_focus(on_view_focus);

        wlc::log_set_rust_handler(log_callback);

        let run_wlc = wlc::init().ok_or("unable to initialize wlc")?;

        Ok(Wayland{
            log: slog_scope::logger().new(o!("wlc" => "wayland backend")),
            run: run_wlc
        })
    }

    fn run(&self) -> Result<()> {
        slog_info!(self.log, "starting backend");

        let run = self.run;
        run();

        Ok(())
    }

    fn stop(&self) {
        slog_info!(self.log, "stopping backend");
        wlc::terminate();
    }
}

impl Wayland {
}
