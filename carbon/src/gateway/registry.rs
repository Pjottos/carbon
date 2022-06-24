use crate::{
    gateway::{client::Clients, message::MessageError},
    protocol::*,
};

use slotmap::SlotMap;

use std::{fmt::Debug, num::NonZeroU32};

slotmap::new_key_type! { pub struct GlobalObjectId; }

pub struct ObjectRegistry {
    display_id: GlobalObjectId,
    registry_id: GlobalObjectId,
    globals: Vec<GlobalObjectId>,
    objects: SlotMap<GlobalObjectId, Option<Interface>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        let mut objects = SlotMap::with_key();

        let display_id = objects.insert(Some(Interface::WlDisplay(WlDisplay)));
        let registry_id = objects.insert(Some(Interface::WlRegistry(WlRegistry)));

        let globals = vec![
            objects.insert(Some(Interface::WlCompositor(WlCompositor))),
            objects.insert(Some(Interface::WlShm(WlShm))),
            objects.insert(Some(Interface::WlDataDeviceManager(WlDataDeviceManager))),
            objects.insert(Some(Interface::WlSubcompositor(WlSubcompositor))),
            objects.insert(Some(Interface::XdgWmBase(XdgWmBase))),
        ];

        Self {
            display_id,
            registry_id,
            globals,
            objects,
        }
    }

    #[inline]
    pub fn display_id(&self) -> GlobalObjectId {
        self.display_id
    }

    #[inline]
    pub fn registry_id(&self) -> GlobalObjectId {
        self.registry_id
    }

    #[inline]
    pub fn globals(&self) -> impl Iterator<Item = (GlobalObjectId, &Interface)> {
        self.globals
            .iter()
            .copied()
            .map(|id| (id, self.get(id).unwrap()))
    }

    pub fn make_global(
        &mut self,
        id: GlobalObjectId,
        clients: &mut Clients,
    ) -> Result<(), MessageError> {
        if self.globals.contains(&id) {
            return Ok(());
        }

        if let Some(new_global) = self.get(id) {
            for (client, id) in clients.find_interface_in_clients(self, |interface| {
                matches!(interface, Interface::WlRegistry(_))
            }) {
                wl_registry::emit_global(
                    client.stream_mut().send_buf_mut(),
                    id,
                    new_global.id(),
                    new_global.name(),
                    new_global.version(),
                )?;
            }

            self.globals.push(id);
        }

        Ok(())
    }

    #[inline]
    pub fn remove_global(
        &mut self,
        id: GlobalObjectId,
        clients: &mut Clients,
    ) -> Result<(), MessageError> {
        if let Some(idx) = self.globals.iter().position(|&g| g == id) {
            let global = self.get(id).unwrap();
            for (client, id) in clients.find_interface_in_clients(self, |interface| {
                matches!(interface, Interface::WlRegistry(_))
            }) {
                wl_registry::emit_global_remove(
                    client.stream_mut().send_buf_mut(),
                    id,
                    global.id(),
                )?;
            }

            self.globals.swap_remove(idx);
        }

        Ok(())
    }

    #[inline]
    pub fn get(&self, id: GlobalObjectId) -> Option<&Interface> {
        self.objects.get(id).and_then(Option::as_ref)
    }

    #[inline]
    pub fn take(&mut self, id: GlobalObjectId) -> Option<Interface> {
        self.objects.get_mut(id).and_then(|b| b.take())
    }

    #[inline]
    pub fn restore(&mut self, id: GlobalObjectId, object: Interface) {
        let entry = self
            .objects
            .get_mut(id)
            .expect("Tried to restore non-existent object");

        *entry = Some(object);
    }

    #[inline]
    pub fn insert(&mut self, object: Interface) -> GlobalObjectId {
        self.objects.insert(Some(object))
    }

    #[inline]
    pub fn remove(&mut self, id: GlobalObjectId) -> Option<Interface> {
        self.objects
            .remove(id)
            .map(|entry| entry.expect("Tried to remove object that was temporarily taken"))
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectId(NonZeroU32);

impl ObjectId {
    #[inline]
    pub fn new(raw: u32) -> Option<Self> {
        NonZeroU32::new(raw).map(Self)
    }

    #[inline]
    pub fn raw(self) -> u32 {
        self.0.get()
    }
}

pub struct ClientObjects {
    objects: Vec<Option<GlobalObjectId>>,
}

impl ClientObjects {
    #[inline]
    pub fn new(display_id: GlobalObjectId) -> Self {
        Self {
            objects: vec![None, Some(display_id)],
        }
    }

    #[inline]
    pub fn get(&self, id: ObjectId) -> Option<GlobalObjectId> {
        self.objects.get(id.0.get() as usize).and_then(|id| *id)
    }

    #[inline]
    pub fn register(
        &mut self,
        id: ObjectId,
        global_id: Option<GlobalObjectId>,
    ) -> Result<(), MessageError> {
        let idx = id.0.get() as usize;
        if idx == self.objects.len() {
            self.objects.push(global_id);
            Ok(())
        } else {
            let entry = self
                .objects
                .get_mut(idx)
                .ok_or(MessageError::InvalidObject)?;
            if entry.is_none() {
                *entry = global_id;
                Ok(())
            } else {
                Err(MessageError::InvalidObject)
            }
        }
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (ObjectId, GlobalObjectId)> + '_ {
        self.objects
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.map(|o| (ObjectId::new(i as u32).unwrap(), o)))
    }
}
