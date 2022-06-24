use crate::{
    gateway::{
        client::Clients,
        registry::{GlobalObjectId, ObjectRegistry},
    },
    protocol::{wl_seat::Capability, Interface, WlSeat},
};

use slotmap::{new_key_type, SlotMap};

new_key_type! { pub struct SeatId; }

pub struct Seat {
    capabilities: Capability,
    object_id: GlobalObjectId,
}

pub struct InputState {
    seats: SlotMap<SeatId, Seat>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            seats: SlotMap::with_key(),
        }
    }
}

pub struct InputSink<'a> {
    pub state: &'a mut InputState,
    pub registry: &'a mut ObjectRegistry,
    pub clients: &'a mut Clients,
}

impl<'a> InputSink<'a> {
    pub fn create_seat(&mut self, capabilities: Capability) -> SeatId {
        let seat = Seat {
            capabilities,
            object_id: GlobalObjectId::default(),
        };
        let seat_id = self.state.seats.insert(seat);

        let wl_seat = WlSeat { id: seat_id };
        let object_id = self.registry.insert(Interface::WlSeat(wl_seat));
        self.registry
            .make_global(object_id, self.clients)
            .expect("send buffer full");
        self.state.seats.get_mut(seat_id).unwrap().object_id = object_id;

        seat_id
    }

    pub fn destroy_seat(&mut self, id: SeatId) {
        if let Some(seat) = self.state.seats.remove(id) {
            self.registry
                .remove_global(seat.object_id, self.clients)
                .expect("send buffer full");
        }
    }
}
