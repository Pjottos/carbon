use super::interface::Interface;

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
