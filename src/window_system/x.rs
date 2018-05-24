use xcb::{self, xkb};
use window_system::*;

pub struct XWindowSystem {
    conn: xcb::Connection,
    screens: i32,
}

fn init_xkb(conn: &xcb::Connection) -> Result<()> {
    info!("initializing xkb");
    conn.prefetch_extension_data(xkb::id());

    conn.get_extension_data(xkb::id())
        .ok_or("XKB extension not supported")?;

    if !xkb::use_extension(conn, 1, 0).get_reply()
        .map_err(|_| "failed to get xkb extension version")?
        .supported() {
            return Err("xkb-1.0 is not supported".into())
    }

    let map_parts = xkb::MAP_PART_KEY_TYPES |
        xkb::MAP_PART_KEY_SYMS |
        xkb::MAP_PART_MODIFIER_MAP |
        xkb::MAP_PART_EXPLICIT_COMPONENTS |
        xkb::MAP_PART_KEY_ACTIONS |
        xkb::MAP_PART_KEY_BEHAVIORS |
        xkb::MAP_PART_VIRTUAL_MODS |
        xkb::MAP_PART_VIRTUAL_MOD_MAP;

    let events = xkb::EVENT_TYPE_NEW_KEYBOARD_NOTIFY |
        xkb::EVENT_TYPE_MAP_NOTIFY |
        xkb::EVENT_TYPE_STATE_NOTIFY;

    let cookie = xkb::select_events_checked(conn,
                                            xkb::ID_USE_CORE_KBD as u16,
                                            events as u16, 0, events as u16,
                                            map_parts as u16, map_parts as u16, None);

    cookie.request_check()
        .map(|_| ())
        .map_err(|_| "failed to select notify events from xkb".into())
}

fn cast_event<'r, T>(evt: &'r xcb::GenericEvent) -> &'r T {
    unsafe {
        xcb::cast_event(evt)
    }
}

impl WindowSystem for XWindowSystem {
    fn initialize() -> Result<XWindowSystem> {
        info!("initializing X backend");

        let (conn, screens) = xcb::Connection::connect(None)?;
        conn.has_error()?;

        {
            let setup = &conn.get_setup();
            let screen: xcb::xproto::Screen = setup.roots().nth(screens as usize)
                .ok_or(Error::from("whatever"))?;

            info!("Screen: {}", screen.root());
            info!("\twidth: {}", screen.width_in_pixels());
            info!("\theight: {}", screen.height_in_pixels());
            info!("\twhite pixel: {:x}", screen.white_pixel());
            info!("\tblack pixel: {}", screen.black_pixel());

            let cookie = xcb::change_window_attributes(&conn, screen.root(),
                &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_SUBSTRUCTURE_NOTIFY |
                xcb::EVENT_MASK_SUBSTRUCTURE_REDIRECT |
                xcb::EVENT_MASK_BUTTON_PRESS |
                xcb::EVENT_MASK_KEY_PRESS |
                xcb::EVENT_MASK_ENTER_WINDOW |
                xcb::EVENT_MASK_EXPOSURE |
                xcb::EVENT_MASK_PROPERTY_CHANGE)]);

            if !cookie.request_check().is_ok() {
                return Err(Error::from("another WM is running"));
            }
        }

        init_xkb(&conn)
            .chain_err(|| "failed to initialize xkb")?;

        Ok(XWindowSystem {
            conn: conn,
            screens: screens,
        })
    }

    fn run(&self) -> Result<()> {
        info!("starting backend");

        loop {
            let event = self.conn.wait_for_event();
            match event {
                Some(event) => {
                    match event.response_type() {
                        xcb::CREATE_NOTIFY => {
                            info!("create notify");
                        },
                        xcb::CONFIGURE_NOTIFY => {
                            info!("configure notify");
                        },
                        xcb::CONFIGURE_REQUEST => {
                            info!("configure request");
                            let evt: &xcb::ConfigureRequestEvent = cast_event(&event);
                        },
                        xcb::MAP_NOTIFY => {
                            info!("map notify");
                        },
                        xcb::MAP_REQUEST => {
                            info!("map request");
                        },
                        xcb::MAP_WINDOW => {
                            info!("map window");
                        },
                        xcb::MAP_SUBWINDOWS => {
                            info!("map subwindow");
                        },
                        xcb::UNMAP_NOTIFY => {
                            info!("unmap notify");
                        },
                        xcb::EXPOSE => {
                            info!("expose event");
                        },
                        xcb::FOCUS_OUT => {
                            info!("focus out");
                        },
                        xcb::DESTROY_NOTIFY => {
                            info!("destroy notify");
                        },
                        xcb::BUTTON_PRESS => {
                            info!("button press");
                        },
                        xcb::KEY_PRESS => {
                            info!("key press");
                        },
                        x => {
                            info!("unknown event of type: {}", x);
                        }
                    }
                },
                None => {
                    error!("I/O error on event loop");
                    break;
                }
            }
        }

        Ok(())
    }

    fn stop(&self) {
        info!("stopping backend");
    }
}
