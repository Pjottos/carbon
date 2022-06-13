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
            Ok(event) => log::info!("{:?}", event),
            Err(e) => panic!("ERROR {:?}", e),
        }
    }
}
