use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use async_std::prelude::FutureExt;
use bevy_ecs::system::Resource;
use cgmath::num_traits::ToBytes;
use futures::{Future, Stream};
use quinn::{Endpoint, ReadExactError, RecvStream, SendStream};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::message::{MessageToClient, MessageToServer};

#[derive(Debug)]
pub struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        server_name: &rustls::ServerName,
        scts: &mut dyn Iterator<Item = &[u8]>,
        ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

pub struct Connection {
    pub player_id: Uuid,
    pub transport: Transport,
}

#[derive(Clone, Resource)]
pub enum Transport {
    // Local(LocalTransport),
    Remote(RemoteTransport),
}

// pub type Transaction<S: Serialize + 'static, R: for<'de> Deserialize<'de>> =
//     impl Stream<Item = anyhow::Result<R>>;

// pub type Responder<R> = impl Future<Output = Result<(), anyhow::Error>>;
// pub type ResponderFunc<S: for <'de> Deserialize<'de> + Debug, R: Serialize + 'static> = impl FnOnce(R) -> Responder<R>;

pub async fn write<T: Serialize>(tx: &mut SendStream, msg: T) -> anyhow::Result<()> {
    // let bytes = serde_json::to_string_pretty(&msg)?;
    let bytes = postcard::to_allocvec(&msg).unwrap();
    tx.write_all(&(bytes.len() as u32).to_be_bytes()).await?;
    tx.write_all(&bytes).await?;
    Ok(())
}

pub async fn read<T: for <'de> Deserialize<'de>>(rx: &mut RecvStream) -> anyhow::Result<T> {
    // println!("Reading length");
    let mut len = [0; 4];
    rx.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    // println!("Length was {len}");
    let mut buffer = vec![0; len];
    // println!("Reading data");
    rx.read_exact(&mut buffer).await?;
    // println!("Data read");
    // println!("{}", std::str::from_utf8(&buffer).unwrap());
    // Ok(serde_json::from_slice(&buffer)?)
    Ok(postcard::from_bytes(&buffer)?)
}

pub struct Transaction<R> {
    rx: RecvStream,
    _r: PhantomData<R>,
}

impl<R: for<'de> Deserialize<'de>> Transaction<R> {
    pub async fn single(&mut self) -> anyhow::Result<R> {
        read(&mut self.rx).await
    }

    pub fn stream(&mut self) -> impl Stream<Item = anyhow::Result<R>> + '_ {
        futures::stream::unfold(&mut self.rx, |rx| async {
            let msg = read(rx).await;
            Some((msg, rx))
        })
    }
}

impl Transport {
    pub async fn transact<S: Serialize + 'static, R: for<'de> Deserialize<'de>>(
        &self,
        msg: &S,
    ) -> anyhow::Result<Transaction<R>> {
        match self {
            Transport::Remote(remote) => {
                let remote = remote.clone();
                let (mut tx, rx) = remote.connection.open_bi().await?;

                write(&mut tx, msg).await?;
                // tx.finish().await?;

                Ok(Transaction {
                    rx,
                    _r: PhantomData,
                })
            }
        }
    }

    pub async fn accept_transact<S: for<'de> Deserialize<'de> + Debug, R: Serialize + 'static>(
        &mut self,
    ) -> anyhow::Result<(S, Respond<R>)> {
        match self {
            Transport::Remote(remote) => {
                let (tx, mut rx) = remote.connection.accept_bi().await?;
                println!("Server accepted stream");
                let msg = read(&mut rx).await?;
                
                println!("Server received message");

                Ok((msg, Respond { tx, _r: PhantomData }))
            }
        }
    }
}

pub struct Respond<R> {
    tx: SendStream,
    _r: PhantomData<R>
}

impl<R: Serialize> Respond<R> {
    pub async fn respond(&mut self, msg: &R) -> anyhow::Result<()> {
        write(&mut self.tx, msg).await?;
        Ok(())
    }
}

pub struct LocalTransport {
    pub rx: std::sync::mpsc::Receiver<MessageToServer>,
    pub tx: std::sync::mpsc::Sender<MessageToClient>,
}

#[derive(Clone)]
pub struct RemoteTransport {
    pub connection: quinn::Connection,
}
