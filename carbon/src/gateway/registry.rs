use super::interface::Interface;

use slotmap::SlotMap;

slotmap::new_key_type! { pub struct GlobalObjectId; }

pub struct ObjectRegistry {
    display_id: GlobalObjectId,
    globals: Vec<GlobalObjectId>,
    objects: SlotMap<GlobalObjectId, Box<dyn Interface>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        let mut objects = SlotMap::with_key();

        let display: Box<dyn Interface> = Box::new(WlDisplay);
        let display_id = objects.insert(display);

        Self {
            display_id,
            globals: vec![],
            objects,
        }
    }

    pub fn display_id(&self) -> GlobalObjectId {
        self.display_id
    }

    pub fn get_mut(&mut self, id: GlobalObjectId) -> Option<&mut (dyn Interface + 'static)> {
        self.objects.get_mut(id).map(|b| &mut **b)
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
        _fds: &mut std::collections::VecDeque<std::os::unix::prelude::RawFd>,
    ) -> Result<(), super::message::MessageError> {
        log::debug!("Would call {} with {:?}", opcode, args);
        Ok(())
    }
}
