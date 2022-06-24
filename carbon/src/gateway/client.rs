use crate::{
    gateway::{
        message::MessageStream,
        registry::{ClientObjects, GlobalObjectId, ObjectId, ObjectRegistry},
    },
    protocol::Interface,
};

pub struct Client {
    stream: MessageStream,
    objects: ClientObjects,
}

impl Client {
    pub fn new(stream: MessageStream, display_id: GlobalObjectId) -> Self {
        Self {
            stream,
            objects: ClientObjects::new(display_id),
        }
    }

    #[inline]
    pub fn stream_and_objects_mut(&mut self) -> (&mut MessageStream, &mut ClientObjects) {
        (&mut self.stream, &mut self.objects)
    }

    #[inline]
    pub fn stream_mut(&mut self) -> &mut MessageStream {
        &mut self.stream
    }
}

pub struct Clients {
    clients: Vec<Option<Client>>,
}

impl Clients {
    pub fn new() -> Self {
        Self { clients: vec![] }
    }

    pub fn next_id(&self) -> u32 {
        self.clients
            .iter()
            .take_while(|c| c.is_some())
            .count()
            .try_into()
            .expect("Too many clients")
    }

    pub fn insert_or_push(&mut self, id: u32, client: Client) {
        match self.clients.get_mut(id as usize) {
            Some(entry) => *entry = Some(client),
            None => self.clients.push(Some(client)),
        }
    }

    pub fn delete(&mut self, id: u32) {
        if let Some(entry) = self.clients.get_mut(id as usize) {
            let _ = entry.take();
        }
    }

    pub fn get_mut(&mut self, id: u32) -> Option<&mut Client> {
        self.clients.get_mut(id as usize).and_then(Option::as_mut)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Client> {
        self.clients.iter_mut().filter_map(Option::as_mut)
    }

    pub fn find_interface_in_clients<'a, F>(
        &'a mut self,
        registry: &'a ObjectRegistry,
        mut filter: F,
    ) -> impl Iterator<Item = (&mut Client, ObjectId)> + 'a
    where
        F: FnMut(&Interface) -> bool + 'a,
    {
        self.iter_mut().filter_map(move |client| {
            let res = client
                .objects
                .iter()
                .find(|&(_, global_id)| registry.get(global_id).map_or(false, &mut filter));
            res.map(|(id, _)| (client, id))
        })
    }
}
