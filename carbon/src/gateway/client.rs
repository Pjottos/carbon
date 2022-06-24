use super::{
    message::MessageStream,
    registry::{ClientObjects, GlobalObjectId},
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

    pub fn stream_and_objects_mut(&mut self) -> (&mut MessageStream, &mut ClientObjects) {
        (&mut self.stream, &mut self.objects)
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
}
