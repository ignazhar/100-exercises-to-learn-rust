// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use core::sync;
use std::sync::mpsc::{sync_channel, Receiver, Sender, SyncSender, TryRecvError, TrySendError};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>,
    channel_capacity: usize,
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, TrySendError<TicketDraft>> {
        let (insert_sender, insert_receiver) = sync_channel(self.channel_capacity);
        match self.sender.send(Command::Insert { draft: draft.clone(), response_channel: insert_sender }) {
            Ok(()) => Ok(insert_receiver.recv().unwrap()),
            Err(_) => Err(TrySendError::Full(draft))
        }
    }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, TrySendError<TicketId>> {
        let (get_sender, get_receiver) = sync_channel(self.channel_capacity);
        match self.sender.send(Command::Get { id: id, response_channel: get_sender }) {
            Ok(()) => Ok(get_receiver.recv().unwrap()),
            Err(_) => Err(TrySendError::Full(id))
        }
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient { sender: sender, channel_capacity: capacity }
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Option<Ticket>>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                let _ = response_channel.send(id);
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                let _ = response_channel.send(ticket.cloned());
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
