// LNP/BP Core Library implementing LNPBP specifications & standards
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

//! BOLT-1. Manages state of the remote peer and handles direct communications
//! with it. Relies on transport layer (BOLT-8-based) protocol.

use amplify::Bipolar;
use std::sync::Arc;
#[cfg(not(feature = "tokio"))]
use std::sync::Mutex;
#[cfg(feature = "tokio")]
use tokio::sync::Mutex;

use crate::lnp::application::Messages;
use crate::lnp::presentation::{
    CreateUnmarshaller, Encode, Error, Unmarshall, Unmarshaller,
};
use crate::lnp::session::{
    self, Accept, Connect, LocalNode, NoEncryption, Session, Split,
    ToNodeEndpoint,
};
use crate::lnp::transport::{ftcp, zmqsocket};
use crate::lnp::LIGHTNING_P2P_DEFAULT_PORT;

pub trait RecvMessage {
    fn recv_message(&mut self) -> Result<Messages, Error>;
}

pub trait SendMessage {
    fn send_message(&mut self, message: Messages) -> Result<usize, Error>;
}

pub struct PeerConnection {
    awaiting_pong: bool,
    unmarshaller: Unmarshaller<Messages>,
    session: Box<dyn Session>,
}

pub struct PeerReceiver {
    awaiting_pong: Arc<Mutex<bool>>,
    unmarshaller: Unmarshaller<Messages>,
    //#[cfg(not(feature = "async"))]
    receiver: Box<dyn session::Input + Send>,
    /* #[cfg(feature = "async")]
     * receiver: Box<dyn AsyncRecvFrame>, */
}

pub struct PeerSender {
    awaiting_pong: Arc<Mutex<bool>>,
    //#[cfg(not(feature = "async"))]
    sender: Box<dyn session::Output + Send>,
    /* #[cfg(feature = "async")]
     * sender: Box<dyn AsyncSendFrame>, */
}

impl PeerConnection {
    pub fn with(session: impl Session + 'static) -> Self {
        let unmarshaller = Messages::create_unmarshaller();
        Self {
            awaiting_pong: false,
            unmarshaller,
            session: Box::new(session),
        }
    }

    pub fn connect(
        remote: impl ToNodeEndpoint,
        local: &LocalNode,
    ) -> Result<Self, Error> {
        let unmarshaller = Messages::create_unmarshaller();
        let endpoint = remote
            .to_node_endpoint(LIGHTNING_P2P_DEFAULT_PORT)
            .ok_or(Error::InvalidEndpoint)?;
        let session = endpoint.connect(local)?;
        Ok(Self {
            session,
            awaiting_pong: false,
            unmarshaller,
        })
    }

    pub fn accept(
        remote: impl ToNodeEndpoint,
        local: &LocalNode,
    ) -> Result<Self, Error> {
        let unmarshaller = Messages::create_unmarshaller();
        let endpoint = remote
            .to_node_endpoint(LIGHTNING_P2P_DEFAULT_PORT)
            .ok_or(Error::InvalidEndpoint)?;
        let session = endpoint.accept(local)?;
        Ok(Self {
            session,
            awaiting_pong: false,
            unmarshaller,
        })
    }
}

impl RecvMessage for PeerConnection {
    fn recv_message(&mut self) -> Result<Messages, Error> {
        let payload = self.session.recv_raw_message()?;
        Ok((&*self.unmarshaller.unmarshall(&payload)?).clone())
    }
}

impl SendMessage for PeerConnection {
    fn send_message(&mut self, message: Messages) -> Result<usize, Error> {
        Ok(self.session.send_raw_message(&message.encode()?)?)
    }
}

impl RecvMessage for PeerReceiver {
    fn recv_message(&mut self) -> Result<Messages, Error> {
        let payload = self.receiver.recv_raw_message()?;
        Ok((&*self.unmarshaller.unmarshall(&payload)?).clone())
    }
}

impl SendMessage for PeerSender {
    fn send_message(&mut self, message: Messages) -> Result<usize, Error> {
        Ok(self.sender.send_raw_message(&message.encode()?)?)
    }
}

impl Bipolar for PeerConnection {
    type Left = PeerReceiver;
    type Right = PeerSender;

    fn join(left: Self::Left, right: Self::Right) -> Self {
        unimplemented!()
    }

    fn split(self) -> (Self::Left, Self::Right) {
        let session = self.session.into_any();
        let (input, output) = if let Some(_) = session
            .downcast_ref::<session::Raw<NoEncryption, ftcp::Connection>>()
        {
            let session = session
                .downcast::<session::Raw<NoEncryption, ftcp::Connection>>()
                .expect(
                    "Must not fail; we just ensured that with downcast_ref",
                );
            (*session).split()
        } else if let Some(_) = session
            .downcast_ref::<session::Raw<NoEncryption, zmqsocket::Connection>>()
        {
            let session = session
                .downcast::<session::Raw<NoEncryption, ftcp::Connection>>()
                .expect(
                    "Must not fail; we just ensured that with downcast_ref",
                );
            (*session).split()
        } else {
            panic!("Impossible to split this type of Session")
        };
        let awaiting_pong = Arc::new(Mutex::new(self.awaiting_pong));
        (
            PeerReceiver {
                receiver: input,
                awaiting_pong: awaiting_pong.clone(),
                unmarshaller: self.unmarshaller,
            },
            PeerSender {
                sender: output,
                awaiting_pong,
            },
        )
    }
}
