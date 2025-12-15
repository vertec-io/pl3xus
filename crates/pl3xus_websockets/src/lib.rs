/// A provider for WebSockets
#[cfg(not(target_arch = "wasm32"))]
pub type WebSocketProvider = native_websocket::NativeWesocketProvider;

/// A provider for WebSockets
#[cfg(target_arch = "wasm32")]
pub type WebSocketProvider = wasm_websocket::WasmWebSocketProvider;

#[cfg(not(target_arch = "wasm32"))]
pub use native_websocket::NetworkSettings;

#[cfg(target_arch = "wasm32")]
pub use wasm_websocket::NetworkSettings;

#[cfg(not(target_arch = "wasm32"))]
mod native_websocket {
    use std::{net::SocketAddr, pin::Pin};

    use async_channel::{Receiver, Sender};
    use async_std::net::{TcpListener, TcpStream};
    use async_trait::async_trait;
    use async_tungstenite::tungstenite::protocol::WebSocketConfig;
    use bevy::prelude::{Deref, DerefMut, Resource};
    use pl3xus::managers::NetworkProvider;
    use pl3xus_common::NetworkPacket;
    use pl3xus_common::error::NetworkError;
    use futures::AsyncReadExt;
    use futures_lite::{AsyncWriteExt, Future, FutureExt, Stream};
    use tracing::{debug, error, info, trace, warn};
    use ws_stream_tungstenite::WsStream;

    /// A provider for WebSockets
    #[derive(Default, Debug)]
    pub struct NativeWesocketProvider;

    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    impl NetworkProvider for NativeWesocketProvider {
        const PROVIDER_NAME: &'static str = "WebSocket";

        type NetworkSettings = NetworkSettings;

        type Socket = WsStream<TcpStream>;

        type ReadHalf = futures::io::ReadHalf<WsStream<TcpStream>>;

        type WriteHalf = futures::io::WriteHalf<WsStream<TcpStream>>;

        type ConnectInfo = url::Url;

        type AcceptInfo = SocketAddr;

        type AcceptStream = OwnedIncoming;

        async fn accept_loop(
            accept_info: Self::AcceptInfo,
            _: Self::NetworkSettings,
        ) -> Result<Self::AcceptStream, NetworkError> {
            info!("[accept_loop] Starting - attempting to bind to {}", accept_info);
            let listener = TcpListener::bind(accept_info)
                .await
                .map_err(NetworkError::Listen)?;
            info!("[accept_loop] Successfully bound to {}", accept_info);
            Ok(OwnedIncoming::new(listener))
        }

        async fn connect_task(
            connect_info: Self::ConnectInfo,
            network_settings: Self::NetworkSettings,
        ) -> Result<Self::Socket, NetworkError> {
            info!("Beginning connection");
            if connect_info.scheme() == "wss" {
                return Err(NetworkError::Error(
                    "WSS connections require the TlsWebSocketProvider. Enable the 'tls' feature and use TlsWebSocketProvider instead".to_string(),
                ));
            }

            let (stream, _response) = async_tungstenite::async_std::connect_async_with_config(
                connect_info.as_str(),
                Some(*network_settings),
            )
            .await
            .map_err(|error| match error {
                async_tungstenite::tungstenite::Error::ConnectionClosed => {
                    NetworkError::Error(String::from("Connection closed"))
                }
                async_tungstenite::tungstenite::Error::AlreadyClosed => {
                    NetworkError::Error(String::from("Connection was already closed"))
                }
                async_tungstenite::tungstenite::Error::Io(io_error) => {
                    NetworkError::Error(format!("Io Error: {}", io_error))
                }
                async_tungstenite::tungstenite::Error::Tls(tls_error) => {
                    NetworkError::Error(format!("Tls Error: {}", tls_error))
                }
                async_tungstenite::tungstenite::Error::Capacity(cap) => {
                    NetworkError::Error(format!("Capacity Error: {}", cap))
                }
                async_tungstenite::tungstenite::Error::Protocol(proto) => {
                    NetworkError::Error(format!("Protocol Error: {}", proto))
                }
                async_tungstenite::tungstenite::Error::WriteBufferFull(buf) => {
                    NetworkError::Error(format!("Write Buffer Full Error: {}", buf))
                }
                async_tungstenite::tungstenite::Error::Utf8 => {
                    NetworkError::Error("Utf8 Error".to_string())
                }
                async_tungstenite::tungstenite::Error::AttackAttempt => {
                    NetworkError::Error("Attack Attempt".to_string())
                }
                async_tungstenite::tungstenite::Error::Url(url) => {
                    NetworkError::Error(format!("Url Error: {}", url))
                }
                async_tungstenite::tungstenite::Error::Http(http) => {
                    NetworkError::Error(format!("HTTP Error: {:?}", http))
                }
                async_tungstenite::tungstenite::Error::HttpFormat(http_format) => {
                    NetworkError::Error(format!("HTTP Format Error: {}", http_format))
                }
            })?;
            info!("Connected!");
            return Ok(WsStream::new(stream));
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            settings: Self::NetworkSettings,
        ) {
            let mut buffer = vec![0; settings.max_message_size.unwrap_or(64 << 20)];
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

                if length > settings.max_message_size.unwrap_or(64 << 20) {
                    error!(
                        "Received too large packet: {} > {}",
                        length,
                        settings.max_message_size.unwrap_or(64 << 20)
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
                        error!("Failed to decode network packet: {:?}", err);
                        error!("Buffer length: {}, first 32 bytes: {:?}", length, &buffer[..length.min(32)]);
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
            combined.split()
        }

        fn channel_capacity(settings: &Self::NetworkSettings) -> usize {
            settings.channel_capacity
        }
    }

    #[derive(Clone, Debug, Resource, Deref, DerefMut)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network, both client and server
    pub struct NetworkSettings {
        #[deref]
        pub websocket_config: WebSocketConfig,
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
                websocket_config: WebSocketConfig::default(),
                channel_capacity: 500,
                channel_warning_threshold: 80,
            }
        }
    }

    /// A special stream for recieving ws connections
    type WsStreamFuture = Pin<Box<dyn Future<Output = Option<WsStream<TcpStream>>>>>;

    pub struct OwnedIncoming {
        inner: TcpListener,
        stream: Option<WsStreamFuture>,
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
        type Item = WsStream<TcpStream>;

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            let incoming = self.get_mut();
            if incoming.stream.is_none() {
                let listener: *const TcpListener = &incoming.inner;
                incoming.stream = Some(Box::pin(async move {
                    let stream = unsafe {
                        listener
                            .as_ref()
                            .expect("Segfault when trying to read listener in OwnedStream")
                    }
                    .accept()
                    .await
                    .map(|(s, _)| s)
                    .ok();

                    let stream: WsStream<TcpStream> = match stream {
                        Some(stream) => {
                            info!("🔌 [ACCEPT] TCP connection accepted, attempting WebSocket handshake...");
                            match async_tungstenite::accept_async(stream).await {
                                Ok(stream) => {
                                    info!("🔌 [ACCEPT] WebSocket handshake successful!");
                                    WsStream::new(stream)
                                }
                                Err(e) => {
                                    error!("🔌 [ACCEPT] WebSocket handshake failed: {:?}", e);
                                    return None;
                                }
                            }
                        }

                        None => return None,
                    };
                    Some(stream)
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
}

#[cfg(target_arch = "wasm32")]
mod wasm_websocket {
    use core::panic;
    use std::{net::SocketAddr, pin::Pin};

    use async_channel::{Receiver, Sender};
    use async_io_stream::IoStream;
    use async_trait::async_trait;
    use bevy::prelude::{Deref, DerefMut, Resource};
    use pl3xus::managers::NetworkProvider;
    use pl3xus_common::NetworkPacket;
    use pl3xus_common::error::NetworkError;
    use futures::AsyncReadExt;
    use futures_lite::{AsyncWriteExt, Future, FutureExt, Stream};
    use tracing::{debug, error, info, trace, warn};
    use ws_stream_wasm::{WsMeta, WsStream, WsStreamIo};

    /// A provider for WebSockets
    #[derive(Default, Debug)]
    pub struct WasmWebSocketProvider;

    #[async_trait(?Send)]
    impl NetworkProvider for WasmWebSocketProvider {
        const PROVIDER_NAME: &'static str = "WebSocket";

        type NetworkSettings = NetworkSettings;

        type Socket = (WsMeta, WsStream);

        type ReadHalf = futures::io::ReadHalf<IoStream<WsStreamIo, Vec<u8>>>;

        type WriteHalf = futures::io::WriteHalf<IoStream<WsStreamIo, Vec<u8>>>;

        type ConnectInfo = url::Url;

        type AcceptInfo = SocketAddr;

        type AcceptStream = OwnedIncoming;

        async fn accept_loop(
            accept_info: Self::AcceptInfo,
            _: Self::NetworkSettings,
        ) -> Result<Self::AcceptStream, NetworkError> {
            panic!("Can't create servers on WASM");
        }

        async fn connect_task(
            connect_info: Self::ConnectInfo,
            network_settings: Self::NetworkSettings,
        ) -> Result<Self::Socket, NetworkError> {
            info!("Beginning connection");
            let stream =
                WsMeta::connect(connect_info, None)
                    .await
                    .map_err(|error| match error {
                        ws_stream_wasm::WsErr::InvalidWsState { supplied } => {
                            NetworkError::Error(format!("Invalid Websocket State: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::ConnectionNotOpen => {
                            NetworkError::Error(format!("Connection Not Open"))
                        }
                        ws_stream_wasm::WsErr::InvalidUrl { supplied } => {
                            NetworkError::Error(format!("Invalid URL: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::InvalidCloseCode { supplied } => {
                            NetworkError::Error(format!("Invalid Close Code: {}", supplied))
                        }
                        ws_stream_wasm::WsErr::ReasonStringToLong => {
                            NetworkError::Error(format!("Reason String To Long"))
                        }
                        ws_stream_wasm::WsErr::ConnectionFailed { event } => {
                            NetworkError::Error(format!("Connection Failed: {:?}", event))
                        }
                        ws_stream_wasm::WsErr::InvalidEncoding => {
                            NetworkError::Error(format!("IOnvalid Encoding"))
                        }
                        ws_stream_wasm::WsErr::CantDecodeBlob => {
                            NetworkError::Error(format!("Cant Decode Blob"))
                        }
                        ws_stream_wasm::WsErr::UnknownDataType => {
                            NetworkError::Error(format!("Unkown Data Type"))
                        }
                        _ => NetworkError::Error(format!("Error in Ws_Stream_Wasm")),
                    })?;
            info!("Connected!");
            return Ok(stream);
        }

        async fn recv_loop(
            mut read_half: Self::ReadHalf,
            messages: Sender<NetworkPacket>,
            settings: Self::NetworkSettings,
        ) {
            let mut buffer = vec![0; settings.max_message_size];
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

                if length > settings.max_message_size {
                    error!(
                        "Received too large packet: {} > {}",
                        length, settings.max_message_size
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
                        error!("Failed to decode network packet: {:?}", err);
                        error!("Buffer length: {}, first 32 bytes: {:?}", length, &buffer[..length.min(32)]);
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
            combined.1.into_io().split()
        }

        fn channel_capacity(settings: &Self::NetworkSettings) -> usize {
            settings.channel_capacity
        }
    }

    #[derive(Clone, Debug, Resource)]
    #[allow(missing_copy_implementations)]
    /// Settings to configure the network, both client and server
    pub struct NetworkSettings {
        pub max_message_size: usize,
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
                max_message_size: 64 << 20,
                channel_capacity: 500,
                channel_warning_threshold: 80,
            }
        }
    }

    /// A dummy struct as WASM is unable to accept connections and act as a server
    pub struct OwnedIncoming;

    impl Stream for OwnedIncoming {
        type Item = (WsMeta, WsStream);

        fn poll_next(
            self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Option<Self::Item>> {
            panic!("WASM does not support servers");
        }
    }
}
