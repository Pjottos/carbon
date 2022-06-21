use crate::protocol::{Interface, WlDisplay};

use slotmap::SlotMap;

use std::{marker::PhantomData, num::NonZeroU32};

slotmap::new_key_type! { pub struct GlobalObjectId; }

pub struct ObjectRegistry {
    display_id: GlobalObjectId,
    globals: Vec<GlobalObjectId>,
    objects: SlotMap<GlobalObjectId, Option<Interface>>,
}

impl ObjectRegistry {
    pub fn new() -> Self {
        let mut objects = SlotMap::with_key();

        let display_id = objects.insert(Some(Interface::WlDisplay(WlDisplay)));

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

#[repr(transparent)]
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

pub struct ClientObjects {
    objects: Vec<Option<GlobalObjectId>>,
}

impl ClientObjects {
    pub fn new(display_id: GlobalObjectId) -> Self {
        Self {
            objects: vec![None, Some(display_id)],
        }
    }

    pub fn get<T>(&self, id: ObjectId<T>) -> Option<GlobalObjectId> {
        self.objects.get(id.value.get() as usize).and_then(|id| *id)
    }
}
