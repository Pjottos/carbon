use crate::{gateway::message::MessageError, protocol::*};

use slotmap::SlotMap;

use std::{fmt::Debug, marker::PhantomData, num::NonZeroU32};

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
            objects.insert(Some(Interface::WlSeat(WlSeat))),
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
}

#[repr(transparent)]
// clippy bug?
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(PartialEq, Eq, Hash)]
pub struct ObjectId<T> {
    value: NonZeroU32,
    phantom: PhantomData<T>,
}

impl<T> ObjectId<T> {
    #[inline]
    pub fn new(raw: u32) -> Option<Self> {
        NonZeroU32::new(raw).map(|value| Self {
            value,
            phantom: PhantomData,
        })
    }

    #[inline]
    pub fn raw(self) -> u32 {
        self.value.get()
    }
}

// Can't derive because then the type argument needs to implement Clone as well
impl<T> Clone for ObjectId<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            phantom: PhantomData,
        }
    }
}
impl<T> Copy for ObjectId<T> {}
impl<T> Debug for ObjectId<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ObjectId").field(&self.value).finish()
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
    pub fn get<T>(&self, id: ObjectId<T>) -> Option<GlobalObjectId> {
        self.objects.get(id.value.get() as usize).and_then(|id| *id)
    }

    #[inline]
    pub fn register<T>(
        &mut self,
        id: ObjectId<T>,
        global_id: Option<GlobalObjectId>,
    ) -> Result<(), MessageError> {
        let idx = id.value.get() as usize;
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
}
