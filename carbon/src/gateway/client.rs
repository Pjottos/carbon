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
