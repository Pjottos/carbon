use xcb::x;

fn main() {
    env_logger::init();

    let (connection, preferred_display) = xcb::Connection::connect_with_extensions(
        None,
        &[xcb::Extension::Composite, xcb::Extension::Dri3],
        &[],
    )
    .expect("Could not connect to X");

    log::info!("Connected to X");

    let screen = connection
        .get_setup()
        .roots()
        .nth(preferred_display as usize)
        .expect("Invalid preferred_display");

    connection
        .send_and_check_request(&x::ChangeWindowAttributes {
            window: screen.root(),
            value_list: &[x::Cw::EventMask(x::EventMask::all())],
        })
        .expect("Failed to change root window attributes, is another WM running?");

    loop {
        match connection.wait_for_event() {
            Ok(xcb::Event::X(event)) => match event {
                x::Event::KeyPress(key) => log::debug!("Key press: {:?}", key.detail()),
                x::Event::KeyRelease(key) => log::debug!("Key release: {:?}", key.detail()),
                x::Event::KeymapNotify(_map) => (),
                x::Event::ButtonPress(_button) => (),
                x::Event::ButtonRelease(_button) => (),
                x::Event::MotionNotify(_motion) => (),
                x::Event::EnterNotify(_enter) => (),
                x::Event::LeaveNotify(_leave) => (),
                x::Event::FocusIn(_focus) => (),
                x::Event::FocusOut(_focus) => (),
                x::Event::Expose(_expose) => (),
                x::Event::GraphicsExposure(_exposure) => (),
                x::Event::NoExposure(_exposure) => (),
                x::Event::VisibilityNotify(_visibility) => (),
                x::Event::CreateNotify(_create) => {
                    log::debug!("Create notify");
                }
                x::Event::DestroyNotify(_destroy) => log::debug!("Destroy notify"),
                x::Event::UnmapNotify(_unmap) => log::debug!("Unmap notify"),
                x::Event::MapNotify(_map) => log::debug!("Map notify"),
                x::Event::MapRequest(map) => {
                    log::debug!("Map request");
                    connection.send_request(&x::MapWindow {
                        window: map.window(),
                    });
                    connection.flush().expect("Failed to flush X connection");
                }
                x::Event::ReparentNotify(_reparent) => (),
                x::Event::ConfigureNotify(_configure) => log::debug!("Configure notify"),
                x::Event::ConfigureRequest(_configure) => {
                    log::debug!("Configure request");
                }
                x::Event::GravityNotify(_gravity) => (),
                x::Event::ResizeRequest(_resize) => (),
                x::Event::CirculateNotify(_circulate) => (),
                x::Event::CirculateRequest(_circulate) => (),
                x::Event::PropertyNotify(_property) => (),
                x::Event::SelectionClear(_selection) => (),
                x::Event::SelectionRequest(_selection) => (),
                x::Event::SelectionNotify(_selection) => (),
                x::Event::ColormapNotify(_colormap) => (),
                x::Event::ClientMessage(_msg) => (),
                x::Event::MappingNotify(_mapping) => (),
            },
            Ok(xcb::Event::Shape(event)) => match event {
                xcb::shape::Event::Notify(notify) => log::debug!("shape notify {:?}", notify),
            },
            Ok(xcb::Event::XFixes(event)) => match event {
                xcb::xfixes::Event::SelectionNotify(_selection) => (),
                xcb::xfixes::Event::CursorNotify(_cursor) => (),
            },
            Ok(xcb::Event::Unknown(event)) => {
                log::warn!("Received unrecognized event: {:?}", event);
            }
            Err(e) => log::error!("ERROR {:?}", e),
        }
    }
}
