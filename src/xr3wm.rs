#![feature(globs, phase)]
#[phase(plugin, link)]

extern crate log;
extern crate xlib;
extern crate xinerama;

use config::get_config;
use workspaces::Workspaces;
use xlib_window_system::XlibWindowSystem;
use xlib_window_system::XlibEvent::{ XMapRequest,
                          XConfigurationNotify,
                          XConfigurationRequest,
                          XDestroy,
                          XUnmapNotify,
                          XEnterNotify,
                          XFocusOut,
                          XKeyPress,
                          XButtonPress};

mod config;
mod keycode;
mod commands;
mod xlib_window_system;
mod workspaces;
mod layout;


fn main() {
  let mut config = get_config();

  let ws = &XlibWindowSystem::new();
  ws.grab_modifier(config.mod_key);

  let mut workspaces = Workspaces::new(&mut config, ws.get_screen_infos().len());

  loop {
    match ws.get_event() {
      XMapRequest(window) => {
        debug!("XMapRequest: {}", window);
        if !workspaces.contains(window) {
          let class = ws.get_class_name(window);
          let mut is_hooked = false;

          for hook in config.manage_hooks.iter() {
            if hook.class_name == class {
              is_hooked = true;
              hook.cmd.call(ws, &mut workspaces, &config, window);
            }
          }

          if !is_hooked {
            workspaces.current_mut().add_window(ws, &config, window);
            workspaces.current_mut().focus_window(ws, &config, window);
          }
        }
      },
      XDestroy(window) => {
        debug!("XDestroy: {}", window);
        workspaces.remove_window(ws, &config, window);
      },
      XUnmapNotify(window, send) => {
        debug!("XUnmapNotify: {}", window);
        if send {
          workspaces.remove_window(ws, &config, window);
        }
      },
      XConfigurationNotify(_) => {
        debug!("XConfigurationNotify");
        workspaces.rescreen(ws, &config);
      },
      XConfigurationRequest(window, changes, mask) => {
        debug!("XConfigurationRequest: {}, {}, {}", window, changes, mask);
        ws.configure_window(window, changes, mask);
      },
      XEnterNotify(window) => {
        debug!("XEnterNotify: {}", window);
        workspaces.focus_window(ws, &config, window);
      },
      XFocusOut(_) => {
        debug!("XFocusOut");
        workspaces.current_mut().unfocus_window(ws, &config);
      },
      XButtonPress(window) => {
        debug!("XButtonPress: {}", window);
        workspaces.focus_window(ws, &config, window);
      },
      XKeyPress(_, mods, key) => {
        debug!("XButtonPress: {}, {}", mods, key);
        let mods = mods & !(config.mod_key | 0b10010);

        for binding in config.keybindings.iter() {
          if binding.mods == mods && binding.key == key {
            binding.cmd.call(ws, &mut workspaces, &config);
          }
        }
      },
      _ => {}
    }

    if let Some(ref mut loghook) = (&mut config).log_hook {
      loghook.call(ws, &workspaces);
    }
  }
}
