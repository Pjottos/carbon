use super::{message::MessageStream, registry::GlobalObjectId};

pub struct Client {
    stream: MessageStream,
    objects: Vec<Option<GlobalObjectId>>,
}

impl Client {
    pub fn new(stream: MessageStream, display_id: GlobalObjectId) -> Self {
        Self {
            stream,
            objects: vec![None, Some(display_id)],
        }
    }

    pub fn stream_and_objects_mut(
        &mut self,
    ) -> (&mut MessageStream, &mut Vec<Option<GlobalObjectId>>) {
        (&mut self.stream, &mut self.objects)
    }
}
