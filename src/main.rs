use xcb::x;

fn main() {
    env_logger::init();

    let (connection, preferred_display) =
        xcb::Connection::connect(None).expect("Could not connect to X");

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
                x::Event::KeyPress(key) => log::info!("key press: {:?}", key),
                x::Event::KeyRelease(key) => log::info!("key release: {:?}", key),
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
                x::Event::CreateNotify(_create) => (),
                x::Event::DestroyNotify(_destroy) => (),
                x::Event::UnmapNotify(_unmap) => (),
                x::Event::MapNotify(_map) => (),
                x::Event::MapRequest(_map) => (),
                x::Event::ReparentNotify(_reparent) => (),
                x::Event::ConfigureNotify(_configure) => (),
                x::Event::ConfigureRequest(_configure) => (),
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
            Ok(xcb::Event::Unknown(event)) => {
                log::warn!("Received unrecognized event: {:?}", event);
            }
            Err(e) => panic!("ERROR {:?}", e),
        }
    }
}
