use std::{net::SocketAddr, pin::Pin};

use crate::{
    NetworkPacket,
    async_channel::{Receiver, Sender},
    async_trait,
    // error::NetworkError,
    managers::NetworkProvider,
};
use async_net::{TcpListener, TcpStream};
use bevy::prelude::Resource;
use pl3xus_common::error::NetworkError;
use futures_lite::{AsyncReadExt, AsyncWriteExt, FutureExt, Stream};
use std::future::Future;
use tracing::{debug, error, info, trace, warn};

#[derive(Default, Debug)]
/// Provides a tcp stream and listener for pl3xus.
pub struct TcpProvider;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl NetworkProvider for TcpProvider {
    const PROVIDER_NAME: &'static str = "TCP";

    type NetworkSettings = NetworkSettings;

    type Socket = TcpStream;

    type ReadHalf = TcpStream;

    type WriteHalf = TcpStream;

    type ConnectInfo = SocketAddr;

    type AcceptInfo = SocketAddr;

    type AcceptStream = OwnedIncoming;

    async fn accept_loop(
        accept_info: Self::AcceptInfo,
        _: Self::NetworkSettings,
    ) -> Result<Self::AcceptStream, NetworkError> {
        let listener = TcpListener::bind(accept_info)
            .await
            .map_err(NetworkError::Listen)?;

        Ok(OwnedIncoming::new(listener))
    }

    async fn connect_task(
        connect_info: Self::ConnectInfo,
        _: Self::NetworkSettings,
    ) -> Result<Self::Socket, NetworkError> {
        info!("Beginning connection");
        let stream = TcpStream::connect(connect_info)
            .await
            .map_err(NetworkError::Connection)?;

        info!("Connected!");

        let addr = stream
            .peer_addr()
            .expect("Could not fetch peer_addr of existing stream");

        debug!("Connected to: {:?}", addr);
        return Ok(stream);
    }

    async fn recv_loop(
        mut read_half: Self::ReadHalf,
        messages: Sender<NetworkPacket>,
        settings: Self::NetworkSettings,
    ) {
        let mut buffer = vec![0; settings.max_packet_length];
        loop {
            info!("Reading message length");
            let length = match read_half.read(&mut buffer[..8]).await {
                Ok(0) => {
                    // EOF, meaning the TCP stream has closed.
                    info!("Client disconnected");
                    // TODO: probably want to do more than just quit the receive task.
                    //       to let pl3xus know that the peer disconnected.
                    break;
                }
                Ok(8) => {
                    let bytes = &buffer[..8];
                    u64::from_le_bytes(
                        bytes
                            .try_into()
                            .expect("Couldn't read bytes from connection!"),
                    ) as usize
                }
                Ok(n) => {
                    error!(
                        "Could not read enough bytes for header. Expected 8, got {}",
                        n
                    );
                    break;
                }
                Err(err) => {
                    error!("Encountered error while fetching length: {}", err);
                    break;
                }
            };
            info!("Message length: {}", length);

            if length > settings.max_packet_length {
                error!(
                    "Received too large packet: {} > {}",
                    length, settings.max_packet_length
                );
                break;
            }

            info!("Reading message into buffer");
            match read_half.read_exact(&mut buffer[..length]).await {
                Ok(()) => (),
                Err(err) => {
                    error!(
                        "Encountered error while fetching stream of length {}: {}",
                        length, err
                    );
                    break;
                }
            }
            info!("Message read");

            let packet: NetworkPacket = match bincode::serde::decode_from_slice(&buffer[..length], bincode::config::standard()) {
                Ok((packet, _)) => packet,
                Err(err) => {
                    error!("Failed to decode network packet from: {}", err);
                    break;
                }
            };

            if messages.send(packet).await.is_err() {
                error!("Failed to send decoded message to pl3xus");
                break;
            }
            info!("Message deserialized and sent to pl3xus");
        }
    }

    async fn send_loop(
        mut write_half: Self::WriteHalf,
        messages: Receiver<NetworkPacket>,
        settings: Self::NetworkSettings,
    ) {
        let warning_threshold = settings.channel_warning_threshold;
        let channel_capacity = settings.channel_capacity;

        while let Ok(first_message) = messages.recv().await {
            // Collect all available messages into a batch
            let mut batch = vec![first_message];

            // Use try_recv() to collect additional messages without blocking
            // This automatically batches messages that arrive in quick succession
            loop {
                match messages.try_recv() {
                    Ok(msg) => batch.push(msg),
                    Err(_) => break, // No more messages available right now
                }
            }

            let batch_size = batch.len();

            // Monitor channel depth and warn if approaching capacity
            let remaining_capacity = messages.capacity().unwrap_or(channel_capacity);
            let current_depth = messages.len();
            let depth_percentage = (current_depth as f32 / remaining_capacity as f32 * 100.0) as u8;

            if depth_percentage >= warning_threshold {
                warn!(
                    "Channel depth at {}% ({}/{} messages). Client may be too slow to keep up!",
                    depth_percentage, current_depth, remaining_capacity
                );
            }

            if batch_size > 1 {
                debug!("Batching {} messages into single write", batch_size);
            }

            // Serialize and combine all messages into a single buffer
            let mut combined_buffer = Vec::new();

            for message in batch {
                let encoded = match bincode::serde::encode_to_vec(&message, bincode::config::standard()) {
                    Ok(encoded) => encoded,
                    Err(err) => {
                        error!("Could not encode packet {:?}: {}", message, err);
                        continue;
                    }
                };

                let len = encoded.len() as u64;

                // Add length prefix and data to combined buffer
                combined_buffer.extend_from_slice(&len.to_le_bytes());
                combined_buffer.extend_from_slice(&encoded);
            }

            if combined_buffer.is_empty() {
                continue; // All messages failed to encode
            }

            trace!("Sending {} bytes ({} messages)", combined_buffer.len(), batch_size);

            // Single write for entire batch
            match write_half.write_all(&combined_buffer).await {
                Ok(_) => {
                    if batch_size > 1 {
                        debug!("Successfully sent batch of {} messages", batch_size);
                    }
                },
                Err(err) => {
                    error!("Could not send batch of {} messages: {}", batch_size, err);
                    break;
                }
            }

            trace!("Successfully written all!");
        }
    }

    fn split(combined: Self::Socket) -> (Self::ReadHalf, Self::WriteHalf) {
        (combined.clone(), combined)
    }

    fn channel_capacity(settings: &Self::NetworkSettings) -> usize {
        settings.channel_capacity
    }
}

#[derive(Clone, Debug, Resource)]
#[allow(missing_copy_implementations)]
/// Settings to configure the network, both client and server
pub struct NetworkSettings {
    /// Maximum packet size in bytes. If a client ever exceeds this size, they will be disconnected
    ///
    /// ## Default
    /// The default is set to 10MiB
    pub max_packet_length: usize,
    /// Channel capacity for outgoing messages per connection (default: 500)
    ///
    /// This controls how many messages can be queued for sending before
    /// old messages are dropped. At 60 FPS, 500 messages = ~8 seconds of buffering.
    ///
    /// For industrial applications with high reliability requirements, consider
    /// increasing to 1000-2000 messages.
    pub channel_capacity: usize,
    /// Warn when channel depth exceeds this percentage (default: 80)
    pub channel_warning_threshold: u8,
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            max_packet_length: 10 * 1024 * 1024,
            channel_capacity: 500,
            channel_warning_threshold: 80,
        }
    }
}

/// A special stream for recieving tcp connections
pub struct OwnedIncoming {
    inner: TcpListener,
    stream: Option<Pin<Box<dyn Future<Output = Option<TcpStream>>>>>,
}

impl OwnedIncoming {
    fn new(listener: TcpListener) -> Self {
        Self {
            inner: listener,
            stream: None,
        }
    }
}

impl Stream for OwnedIncoming {
    type Item = TcpStream;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let incoming = self.get_mut();
        if incoming.stream.is_none() {
            let listener: *const TcpListener = &incoming.inner;
            incoming.stream = Some(Box::pin(async move {
                unsafe {
                    listener
                        .as_ref()
                        .expect("Segfault when trying to read listener in OwnedStream")
                }
                .accept()
                .await
                .map(|(s, _)| s)
                .ok()
            }));
        }
        if let Some(stream) = &mut incoming.stream
             && let std::task::Poll::Ready(res) = stream.poll(cx) {
                incoming.stream = None;
                return std::task::Poll::Ready(res);
        }
        std::task::Poll::Pending
    }
}

unsafe impl Send for OwnedIncoming {}
