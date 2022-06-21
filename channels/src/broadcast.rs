pub struct UnboundedBroadcast<T> {
    channels: Vec<crossbeam_channel::Sender<T>>,
}

impl<T: 'static + Clone + Send + Sync> UnboundedBroadcast<T> {
    pub fn new() -> Self {
        // we often create at most, or at least 2 subscribers, so
        // preallocate capacity of 2 as small optimization
        Self {
            channels: Vec::with_capacity(2),
        }
    }

    pub fn subscribe(&mut self) -> crossbeam_channel::Receiver<T> {
        let (tx, rx) = crossbeam_channel::unbounded();

        self.channels.push(tx);

        rx
    }

    pub fn send(&self, message: T) -> Result<(), crossbeam_channel::SendError<T>> {
        for c in self.channels.iter() {
            c.send(message.clone())?;
        }

        Ok(())
    }
}

impl<T: 'static + Clone + Send + Sync> Default for UnboundedBroadcast<T> {
    fn default() -> Self {
        Self::new()
    }
}
