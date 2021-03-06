#![allow(non_upper_case_globals, unused_variables, dead_code)]
#![allow(clippy::too_many_arguments)]

extern crate libc;

use keycode::{MOD_2, MOD_LOCK};
use layout::Rect;
use std::cmp;
use std::str;
use std::env;
use std::default::Default;
use std::ptr::null_mut;
use std::mem::MaybeUninit;
use std::slice::from_raw_parts;
use std::ffi::{CStr, CString};
use self::libc::{c_void, c_uchar, c_int, c_uint, c_long, c_ulong};
use self::libc::malloc;
use self::XlibEvent::*;
use xinerama::XineramaQueryScreens;
use xlib::*;

extern "C" fn error_handler(display: *mut Display, event: *mut XErrorEvent) -> c_int {
    // TODO: proper error handling
    // HACK: fixes LeaveNotify on invalid windows
    0
}

pub struct XlibWindowSystem {
    display: *mut Display,
    root: Window,
    event: *mut c_void,
}

pub enum XlibEvent {
    XMapRequest(Window),
    XConfigurationNotify(Window),
    XConfigurationRequest(Window, WindowChanges, u32),
    XDestroy(Window),
    XUnmapNotify(Window, bool),
    XPropertyNotify(Window, u64, bool),
    XEnterNotify(Window),
    XFocusOut(Window),
    XKeyPress(Window, u8, String),
    XButtonPress(Window),
    Ignored,
}

pub struct SizeHint {
    pub min: Option<(u32, u32)>,
    pub max: Option<(u32, u32)>,
}

pub struct Strut(pub u32, pub u32, pub u32, pub u32);

pub struct WindowChanges {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub border_width: u32,
    pub sibling: Window,
    pub stack_mode: u32,
}

impl XlibWindowSystem {
    pub fn new() -> XlibWindowSystem {
        unsafe {
            let display = XOpenDisplay(null_mut());
            if display.is_null() {
                error!("Can't open display {}",
                       env::var("DISPLAY").unwrap_or_else(|_| "undefined".to_string()));
                panic!();
            }

            let root = XDefaultRootWindow(display);
            XSelectInput(display, root, 0x001A_0034);
            XDefineCursor(display, root, XCreateFontCursor(display, 68));
            XSetErrorHandler(error_handler as *mut u8);

            XlibWindowSystem {
                display,
                root,
                event: malloc(256),
            }
        }
    }

    pub fn close(&self) {
        unsafe {
            XCloseDisplay(self.display);
        }
    }

    pub fn setup_window(&self,
                        x: u32,
                        y: u32,
                        width: u32,
                        height: u32,
                        border_width: u32,
                        border_color: u32,
                        window: Window) {
        self.set_window_border_width(window, border_width);
        self.set_window_border_color(window, border_color);
        self.move_resize_window(window,
                                x,
                                y,
                                cmp::max(width as i32 - (2 * border_width as i32), 0) as u32,
                                cmp::max(height as i32 - (2 * border_width as i32), 0) as u32);
    }

    fn get_property(&self, window: Window, property: u64) -> Option<Vec<u64>> {
        unsafe {
            let mut ret_type: c_ulong = 0;
            let mut ret_format: c_int = 0;
            let mut ret_nitems: c_ulong = 0;
            let mut ret_bytes_after: c_ulong = 0;
            let mut ret_prop = MaybeUninit::<*mut c_ulong>::uninit();

            if XGetWindowProperty(self.display,
                                  window,
                                  property,
                                  0,
                                  0xFFFF_FFFF,
                                  0,
                                  0,
                                  &mut ret_type,
                                  &mut ret_format,
                                  &mut ret_nitems,
                                  &mut ret_bytes_after,
                                  ret_prop.as_mut_ptr() as *mut *mut c_uchar) == 0 {
                if ret_format != 0 {
                    Some(from_raw_parts(ret_prop.assume_init() as *const c_ulong, ret_nitems as usize)
                        .iter()
                        .map(|&x| x as u64)
                        .collect())
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    pub fn get_atom(&self, s: &str) -> u64 {
        unsafe {
            XInternAtom(self.display,
                        CString::new(s.as_bytes())
                            .unwrap()
                            .as_bytes_with_nul()
                            .as_ptr() as *mut i8,
                        0) as u64
        }
    }

    pub fn get_windows(&self) -> Vec<Window> {
        unsafe {
            let mut ret_root: c_ulong = 0;
            let mut ret_parent: c_ulong = 0;
            let mut ret_nchildren: c_uint = 0;
            let mut ret_children = MaybeUninit::<*mut c_ulong>::uninit();

            XQueryTree(self.display,
                       self.root,
                       &mut ret_root,
                       &mut ret_parent,
                       ret_children.as_mut_ptr(),
                       &mut ret_nchildren);

            from_raw_parts(ret_children.assume_init(), ret_nchildren as usize)
                .iter()
                .map(|&x| x as u64)
                .collect()
        }
    }

    pub fn get_strut(&self, screen: Rect) -> Strut {
        let atom = self.get_atom("_NET_WM_STRUT_PARTIAL");

        self.get_windows()
            .iter()
            .filter_map(|&w| {
                self.get_property(w, atom)
            })
            .filter(|x| {
                let screen_x = u64::from(screen.x);
                let screen_y = u64::from(screen.y);
                let screen_height = u64::from(screen.height);
                let screen_width = u64::from(screen.width);

                (x[0] > 0 &&
                 ((x[4] >= screen_y && x[4] < screen_y + screen_height) ||
                  (x[5] >= screen_y && x[5] <= screen_y + screen_height))) ||
                (x[1] > 0 &&
                 ((x[6] >= screen_y && x[6] < screen_y + screen_height) ||
                  (x[7] >= screen_y && x[7] <= screen_y + screen_height))) ||
                (x[2] > 0 &&
                 ((x[8] >= screen_x && x[8] < screen_x + screen_width) ||
                  (x[9] >= screen_x && x[9] <= screen_x + screen_width))) ||
                (x[3] > 0 &&
                 ((x[10] >= screen_x && x[10] < screen_x + screen_width) ||
                  (x[11] >= screen_x && x[11] <= screen_x + screen_width)))
            })
            .map(|x| Strut(x[0] as u32, x[1] as u32, x[2] as u32, x[3] as u32))
            .fold(Strut(0, 0, 0, 0), |a, b| {
                Strut(cmp::max(a.0, b.0),
                      cmp::max(a.1, b.1),
                      cmp::max(a.2, b.2),
                      cmp::max(a.3, b.3))
            })
    }

    fn change_property(&self,
                       window: Window,
                       property: u64,
                       typ: u64,
                       mode: c_int,
                       dat: &mut [c_ulong]) {
        unsafe {
            let ptr: *mut u8 = dat.as_mut_ptr() as *mut u8;
            XChangeProperty(self.display,
                            window,
                            property as c_ulong,
                            typ as c_ulong,
                            32,
                            mode,
                            ptr,
                            2);
        }
    }

    pub fn configure_window(&self,
                            window: Window,
                            window_changes: WindowChanges,
                            mask: u32,
                            unmanaged: bool) {
        unsafe {
            if unmanaged {
                let mut ret_window_changes = XWindowChanges {
                    x: window_changes.x as i32,
                    y: window_changes.y as i32,
                    width: window_changes.width as i32,
                    height: window_changes.height as i32,
                    border_width: window_changes.border_width as i32,
                    sibling: window_changes.sibling,
                    stack_mode: window_changes.stack_mode as i32,
                };
                XConfigureWindow(self.display, window, mask, &mut ret_window_changes);
            } else {
                let rect = self.get_geometry(window);
                let mut attributes = MaybeUninit::uninit();

                XGetWindowAttributes(self.display, window, attributes.as_mut_ptr());

                let mut event = XConfigureEvent {
                    _type: ConfigureRequest as i32,
                    display: self.display,
                    serial: 0,
                    send_event: 1,
                    x: rect.x as i32,
                    y: rect.y as i32,
                    width: rect.width as i32,
                    height: rect.height as i32,
                    border_width: 0,
                    event: window,
                    window,
                    above: 0,
                    override_redirect: attributes.assume_init().override_redirect,
                };
                let event_ptr: *mut XConfigureEvent = &mut event;
                XSendEvent(self.display, window, 0, 0, event_ptr as *mut c_void);
            }
            XSync(self.display, 0);
        }
    }

    pub fn show_window(&self, window: Window) {
        unsafe {
            let atom = self.get_atom("WM_STATE");
            self.change_property(window, atom, atom, 0, &mut [1, 0]);
            XMapWindow(self.display, window);
        }
    }

    pub fn hide_window(&self, window: Window) {
        unsafe {
            XSelectInput(self.display, window, 0x0040_0010);
            XUnmapWindow(self.display, window);
            XSelectInput(self.display, window, 0x0042_0010);

            let atom = self.get_atom("WM_STATE");
            self.change_property(window as u64, atom, atom, 0, &mut [3, 0]);
        }
    }

    pub fn lower_window(&self, window: Window) {
        unsafe {
            XLowerWindow(self.display, window);
        }
    }

    pub fn raise_window(&self, window: Window) {
        unsafe {
            XRaiseWindow(self.display, window);
        }
    }

    pub fn unmap_window(&self, window: Window) {
        unsafe {
            XUnmapWindow(self.display, window);
        }
    }

    pub fn move_resize_window(&self, window: Window, x: u32, y: u32, width: u32, height: u32) {
        unsafe {
            XMoveResizeWindow(self.display, window, x as i32, y as i32, width, height);
        }
    }

    pub fn focus_window(&self, window: Window, color: u32) {
        unsafe {
            XSetInputFocus(self.display, window, 1, 0);
            self.set_window_border_color(window, color);
            XSync(self.display, 0);
        }
    }

    pub fn skip_enter_events(&self) {
        unsafe {
            let event: *mut c_void = malloc(256);
            XSync(self.display, 0);
            while XCheckMaskEvent(self.display, 16, event) != 0 {
            }
        }
    }

    fn has_protocol(&self, window: Window, protocol: &str) -> bool {
        unsafe {
            let mut count = MaybeUninit::uninit();
            let mut atoms = MaybeUninit::uninit();

            XGetWMProtocols(self.display, window, atoms.as_mut_ptr(), count.as_mut_ptr());
            from_raw_parts(atoms.assume_init() as *const c_ulong, count.assume_init() as usize)
                .contains(&self.get_atom(protocol))
        }
    }

    pub fn kill_window(&self, window: Window) {
        if window == 0 {
            return;
        }

        unsafe {
            if self.has_protocol(window, "WM_DELETE_WINDOW") {
                let event = XClientMessageEvent {
                    serial: 0,
                    send_event: 0,
                    _type: 33,
                    format: 32,
                    display: self.display,
                    window,
                    message_type: self.get_atom("WM_PROTOCOLS") as c_ulong,
                    data: [((self.get_atom("WM_DELETE_WINDOW") & 0xFFFF_FFFF_0000_0000) >> 32) as i32,
                           (self.get_atom("WM_DELETE_WINDOW") & 0xFFFF_FFFF) as i32,
                           0,
                           0,
                           0],
                };

                XSendEvent(self.display, window, 0, 0, &event as *const _ as *mut c_void);
            } else {
                XKillClient(self.display, window);
            }
        }
    }

    pub fn restack_windows(&self, mut windows: Vec<Window>) {
        unsafe {
            for w in windows.iter() {
                debug!("{}", w);
            }
            XRestackWindows(self.display,
                            (&mut windows[..]).as_mut_ptr(),
                            windows.len() as i32);
        }
    }

    pub fn grab_button(&self, window: Window) {
        unsafe {
            XGrabButton(self.display, 1, 0x8000, window, 1, 256, 0, 0, 0, 0);
        }
    }

    pub fn grab_modifier(&self, mod_key: u8) {
        unsafe {
            XGrabKey(self.display, 0, u32::from(mod_key), self.root, 1, 0, 1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_2),
                     self.root,
                     1,
                     0,
                     1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_LOCK),
                     self.root,
                     1,
                     0,
                     1);
            XGrabKey(self.display,
                     0,
                     u32::from(mod_key | MOD_2 | MOD_LOCK),
                     self.root,
                     1,
                     0,
                     1);
        }
    }

    pub fn keycode_to_string(&self, keycode: u32) -> String {
        unsafe {
            let keysym = XKeycodeToKeysym(self.display, keycode as u8, 0);
            str::from_utf8(CStr::from_ptr(XKeysymToString(keysym) as *const i8).to_bytes())
                .unwrap()
                .to_string()
        }
    }

    pub fn set_window_border_width(&self, window: Window, width: u32) {
        if window != self.root {
            unsafe {
                XSetWindowBorderWidth(self.display, window, width);
            }
        }
    }

    pub fn set_window_border_color(&self, window: Window, color: u32) {
        if window != self.root {
            unsafe {
                XSetWindowBorder(self.display, window, u64::from(color));
            }
        }
    }

    pub fn get_display_width(&self, screen: u32) -> u32 {
        unsafe { XDisplayWidth(self.display, screen as i32) as u32 }
    }

    pub fn get_display_height(&self, screen: u32) -> u32 {
        unsafe { XDisplayHeight(self.display, screen as i32) as u32 }
    }

    pub fn get_display_rect(&self) -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: self.get_display_width(0),
            height: self.get_display_height(0),
        }
    }

    pub fn get_geometry(&self, window: Window) -> Rect {
        unsafe {
            let mut root = MaybeUninit::uninit();
            let mut x = MaybeUninit::uninit();
            let mut y = MaybeUninit::uninit();
            let mut width = MaybeUninit::uninit();
            let mut height = MaybeUninit::uninit();
            let mut depth = MaybeUninit::uninit();
            let mut border = MaybeUninit::uninit();

            XGetGeometry(self.display,
                         window,
                         root.as_mut_ptr(),
                         x.as_mut_ptr(),
                         y.as_mut_ptr(),
                         width.as_mut_ptr(),
                         height.as_mut_ptr(),
                         border.as_mut_ptr(),
                         depth.as_mut_ptr());

            Rect {
                x: x.assume_init() as u32,
                y: y.assume_init() as u32,
                width: width.assume_init(),
                height: height.assume_init(),
            }
        }
    }

    pub fn get_screen_infos(&self) -> Vec<Rect> {
        unsafe {
            let mut num: c_int = 0;
            let screen_ptr = XineramaQueryScreens(self.display, &mut num);

            if num == 0 {
                return vec![self.get_display_rect()];
            }

            from_raw_parts(screen_ptr, num as usize)
                .iter()
                .map(|screen_info| {
                    Rect {
                        x: screen_info.x_org as u32,
                        y: screen_info.y_org as u32,
                        width: screen_info.width as u32,
                        height: screen_info.height as u32,
                    }
                })
                .collect()
        }
    }

    pub fn is_window_floating(&self, window: Window) -> bool {
        if self.transient_for(window).is_some() {
            return true;
        }

        let hints = self.get_size_hints(window);
        let min = hints.min;
        let max = hints.max;

        if min.is_some() && max.is_some() && min.unwrap().0 == max.unwrap().0 &&
           min.unwrap().1 == max.unwrap().1 {
            return true;
        }

        if let Some(property) = self.get_property(window, self.get_atom("_NET_WM_WINDOW_TYPE")) {
            let dialog = self.get_atom("_NET_WM_WINDOW_TYPE_DIALOG");
            let splash = self.get_atom("_NET_WM_WINDOW_TYPE_SPLASH");

            property.iter().any(|&x| x == dialog || x == splash)
        } else {
            false
        }
    }

    pub fn transient_for(&self, window: Window) -> Option<Window> {
        unsafe {
            let mut w = MaybeUninit::uninit();

            if XGetTransientForHint(self.display, window, w.as_mut_ptr()) != 0 {
                Some(w.assume_init())
            } else {
                None
            }
        }
    }

    pub fn get_size_hints(&self, window: Window) -> SizeHint {
        unsafe {
            let mut size_hint = MaybeUninit::uninit();
            let mut tmp: c_long = 0;
            XGetWMNormalHints(self.display, window, size_hint.as_mut_ptr(), &mut tmp);

            let size_hint = size_hint.assume_init();
            let min = if size_hint.flags.contains(XSizeHintFlags::PMinSize) {
                Some((size_hint.min_width as u32, size_hint.min_height as u32))
            } else {
                None
            };

            let max = if size_hint.flags.contains(XSizeHintFlags::PMaxSize) {
                Some((size_hint.max_width as u32, size_hint.max_height as u32))
            } else {
                None
            };
            SizeHint {
                min,
                max,
            }
        }
    }

    fn get_wm_hints(&self, window: Window) -> &XWMHints {
        unsafe { &*XGetWMHints(self.display, window) }
    }

    pub fn is_urgent(&self, window: Window) -> bool {
        let hints = self.get_wm_hints(window);
        hints.flags.contains(XWMHintFlags::Urgency)
    }

    pub fn get_class_name(&self, window: Window) -> String {
        unsafe {
            let mut hint = MaybeUninit::uninit();

            if XGetClassHint(self.display, window, hint.as_mut_ptr()) != 0 {
                let hint = hint.assume_init();
                if !hint.res_class.is_null() {
                    return match str::from_utf8(CStr::from_ptr(hint.res_class).to_bytes()) {
                        Ok(s) => s.to_string(),
                        Err(_) => String::new(),
                    }
                }
            }
            String::new()
        }
    }

    pub fn get_window_title(&self, window: Window) -> String {
        if window == self.root {
            return String::new();
        }

        unsafe {
            let mut name = MaybeUninit::uninit();
            if XFetchName(self.display, window, name.as_mut_ptr()) != 0 {
                let name = name.assume_init();
                if !name.is_null() {
                    return match str::from_utf8(CStr::from_ptr(name).to_bytes()) {
                        Ok(s) => s.to_string(),
                        Err(_) => String::new(),
                    }
                }
            }
            String::new()
        }
    }

    pub fn move_pointer(&self, x: i32, y: i32) {
        unsafe {
            let mut root_w = MaybeUninit::uninit();
            let mut child_w = MaybeUninit::uninit();
            let mut root_x = MaybeUninit::uninit();
            let mut root_y = MaybeUninit::uninit();
            let mut win_x = MaybeUninit::uninit();
            let mut win_y = MaybeUninit::uninit();
            let mut mask = MaybeUninit::uninit();

            let ret = XQueryPointer(
                self.display,
                self.root,
                root_w.as_mut_ptr() as *mut Window,
                child_w.as_mut_ptr() as *mut Window,
                root_x.as_mut_ptr() as *mut i32,
                root_y.as_mut_ptr() as *mut i32,
                win_x.as_mut_ptr() as *mut i32,
                win_y.as_mut_ptr() as *mut i32,
                mask.as_mut_ptr() as *mut u32);

            if ret == 1 {
                XWarpPointer(self.display, 0, 0, 0, 0, 0, 0, x - root_x.assume_init(), y - root_y.assume_init());
            }
        }
    }

    fn cast_event_to<T>(&self) -> &T {
        unsafe { &*(self.event as *const T) }
    }

    pub fn get_event(&self) -> XlibEvent {
        unsafe {
            XNextEvent(self.display, self.event);
        }

        let evt_type: c_int = *self.cast_event_to();
        match evt_type {
            MapRequest => {
                let evt: &XMapRequestEvent = self.cast_event_to();

                unsafe {
                    let atom = self.get_atom("WM_STATE");
                    self.change_property(evt.window as u64, atom, atom, 0, &mut [1, 0]);
                    self.grab_button(evt.window);
                    XSelectInput(self.display, evt.window, 0x0042_0010);
                }

                XMapRequest(evt.window)
            }
            ConfigureNotify => {
                let evt: &XConfigureEvent = self.cast_event_to();
                if evt.window == self.root {
                    XConfigurationNotify(evt.window)
                } else {
                    Ignored
                }
            }
            ConfigureRequest => {
                let event: &XConfigureRequestEvent = self.cast_event_to();
                let changes = WindowChanges {
                    x: event.x as u32,
                    y: event.y as u32,
                    width: event.width as u32,
                    height: event.height as u32,
                    border_width: event.border_width as u32,
                    sibling: event.above as Window,
                    stack_mode: event.detail as u32,
                };
                XConfigurationRequest(event.window, changes, event.value_mask as u32)
            }
            DestroyNotify => {
                let evt: &XDestroyWindowEvent = self.cast_event_to();
                XDestroy(evt.window)
            }
            UnmapNotify => {
                let evt: &XUnmapEvent = self.cast_event_to();
                XUnmapNotify(evt.window, evt.send_event > 0)
            }
            PropertyNotify => {
                let evt: &XPropertyEvent = self.cast_event_to();
                XPropertyNotify(evt.window, evt.atom, evt.state == 0)
            }
            EnterNotify => {
                let evt: &XEnterWindowEvent = self.cast_event_to();
                if evt.detail != 2 {
                    XEnterNotify(evt.window)
                } else {
                    Ignored
                }
            }
            FocusOut => {
                let evt: &XFocusOutEvent = self.cast_event_to();
                if evt.detail != 5 {
                    XFocusOut(evt.window)
                } else {
                    Ignored
                }
            }
            ButtonPress => {
                let evt: &XButtonPressedEvent = self.cast_event_to();
                unsafe {
                    XAllowEvents(self.display, 2, 0);
                }

                XButtonPress(evt.window)
            }
            KeyPress => {
                let evt: &XKeyPressedEvent = self.cast_event_to();
                XKeyPress(evt.window,
                          evt.state as u8,
                          self.keycode_to_string(evt.keycode))
            }
            _ => Ignored,
        }
    }
}

impl Default for XlibWindowSystem {
    fn default() -> Self {
        Self::new()
    }
}
