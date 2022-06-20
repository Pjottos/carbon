use super::{
    interface::{DispatchState, Interface},
    message::MessageError,
};

use slotmap::SlotMap;

slotmap::new_key_type! { pub struct GlobalObjectId; }

pub struct ObjectRegistry {
    display_id: GlobalObjectId,
    globals: Vec<GlobalObjectId>,
    objects: SlotMap<GlobalObjectId, Option<Interface>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        let mut objects = SlotMap::with_key();

        let display_id = objects.insert(Some(Interface::WlDisplay));

        Self {
            display_id,
            globals: vec![],
            objects,
        }
    }

    pub fn display_id(&self) -> GlobalObjectId {
        self.display_id
    }

    pub fn take(&mut self, id: GlobalObjectId) -> Option<Interface> {
        self.objects.get_mut(id).and_then(|b| b.take())
    }

    pub fn restore(&mut self, id: GlobalObjectId, object: Interface) {
        let entry = self
            .objects
            .get_mut(id)
            .expect("Tried to restore non-existent object");

        *entry = Some(object);
    }
}

// TODO: automatically generate this
struct WlDisplay;

pub type RequestDemarshaller = fn(&[u32], &mut DispatchState) -> Result<(), MessageError>;

pub static INTERFACE_NAMES: [&str; 1] = ["wl_display"];
pub static INTERFACE_VERSIONS: [u32; 1] = [1];
pub static INTERFACE_DISPATCH_TABLE: [[Option<RequestDemarshaller>; 2]; 1] =
    [[Some(wl_display::sync), Some(wl_display::get_registry)]];

mod wl_display {
    use super::*;

    pub fn sync(args: &[u32], state: &mut DispatchState) -> Result<(), MessageError> {
        let buf = state.send_buf.allocate(3).unwrap();
        buf[0] = args[0];
        buf[1] = 0x000C0000;
        buf[2] = 0;

        Ok(())
    }

    pub fn get_registry(_args: &[u32], _state: &mut DispatchState) -> Result<(), MessageError> {
        Ok(())
    }
}
