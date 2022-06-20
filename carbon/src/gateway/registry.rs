use super::interface::{DispatchState, Interface};

use slotmap::SlotMap;

slotmap::new_key_type! { pub struct GlobalObjectId; }

pub struct ObjectRegistry {
    display_id: GlobalObjectId,
    globals: Vec<GlobalObjectId>,
    objects: SlotMap<GlobalObjectId, Option<Box<dyn Interface>>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        let mut objects = SlotMap::with_key();

        let display: Box<dyn Interface> = Box::new(WlDisplay);
        let display_id = objects.insert(Some(display));

        Self {
            display_id,
            globals: vec![],
            objects,
        }
    }

    pub fn display_id(&self) -> GlobalObjectId {
        self.display_id
    }

    pub fn take(&mut self, id: GlobalObjectId) -> Option<Box<dyn Interface>> {
        self.objects.get_mut(id).and_then(|b| b.take())
    }

    pub fn restore(&mut self, id: GlobalObjectId, object: Box<dyn Interface>) {
        let entry = self
            .objects
            .get_mut(id)
            .expect("Tried to restore non-existent object");

        *entry = Some(object);
    }
}

// TODO: automatically generate this
struct WlDisplay;

impl Interface for WlDisplay {
    fn name(&self) -> &'static str {
        "wl_display"
    }

    fn version(&self) -> u32 {
        1
    }

    fn dispatch(
        &mut self,
        opcode: u16,
        args: &[u32],
        state: &mut DispatchState,
    ) -> Result<(), super::message::MessageError> {
        if opcode == 0 {
            let buf = state.send_buf.allocate(3).unwrap();
            buf[0] = args[0];
            buf[1] = 0x000C0000;
            buf[2] = 0;
        }

        Ok(())
    }
}
