use crate::{
    gateway::registry::ObjectRegistry,
    protocol::{wl_seat::Capability, Interface, WlSeat},
};

pub struct InputState {
    seats: Vec<Option<Seat>>,
}

#[derive(Debug, Clone, Copy)]
pub struct SeatId(u32);

pub struct Seat {
    capabilities: Capability,
}

impl InputState {
    pub fn new() -> Self {
        Self { seats: vec![] }
    }

    fn insert_seat(&mut self, seat: Seat) -> SeatId {
        match self.seats.iter_mut().enumerate().find(|(_, e)| e.is_none()) {
            Some((idx, entry)) => {
                *entry = Some(seat);
                SeatId(idx as u32)
            }
            None => {
                let idx = self.seats.len();
                self.seats.push(Some(seat));
                SeatId(idx as u32)
            }
        }
    }
}

pub struct InputSink<'a> {
    pub state: &'a mut InputState,
    pub registry: &'a mut ObjectRegistry,
}

impl<'a> InputSink<'a> {
    pub fn create_seat(&mut self, capabilities: Capability) -> SeatId {
        let seat = Seat { capabilities };
        let seat_id = self.state.insert_seat(seat);

        let wl_seat = WlSeat { id: seat_id };
        let object_id = self.registry.insert(Interface::WlSeat(wl_seat));
        self.registry.make_object_global(object_id);

        log::warn!("TODO: broadcast new seat to connected clients");

        seat_id
    }
}
