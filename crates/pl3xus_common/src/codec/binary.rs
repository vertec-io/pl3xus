use codee::{Decoder, Encoder};
use serde::de::DeserializeOwned;

use crate::{error::NetworkError, Pl3xusMessage, NetworkPacket};

/// Primary codec for pl3xus WebSocket communication (multi-message mode).
///
/// This codec handles the pl3xus WebSocket protocol:
/// - 8-byte little-endian length prefix
/// - Bincode-serialized NetworkPacket
///
/// This is the **recommended codec** for production applications as it supports
/// multiple message types over a single WebSocket connection.
///
/// ## Usage
///
/// ```rust,ignore
/// use pl3xus_common::codec::Pl3xusBincodeCodec;
/// use pl3xus_common::NetworkPacket;
///
/// let ws = use_websocket_with_options::<NetworkPacket, NetworkPacket, Pl3xusBincodeCodec>(
///     "ws://127.0.0.1:8081",
///     options
/// );
/// ```
///
/// The application layer is responsible for:
/// - Wrapping messages in NetworkPacket before sending
/// - Unwrapping NetworkPacket after receiving
/// - Routing messages based on NetworkPacket.type_name
pub struct Pl3xusBincodeCodec;

// Multi-message encoder: accepts NetworkPacket directly (already wrapped by application)
impl Encoder<NetworkPacket> for Pl3xusBincodeCodec {
    type Error = NetworkError;
    type Encoded = Vec<u8>;

    fn encode(val: &NetworkPacket) -> Result<Self::Encoded, Self::Error> {
        // The NetworkPacket is already created by the application layer
        // We just need to encode it with bincode and add the length prefix
        let encoded_packet = bincode::serde::encode_to_vec(val, bincode::config::standard())
            .map_err(|_| NetworkError::Serialization)?;

        let len = encoded_packet.len() as u64;
        let mut buffer = Vec::with_capacity(8 + encoded_packet.len());
        buffer.extend_from_slice(&len.to_le_bytes());
        buffer.extend_from_slice(&encoded_packet);

        Ok(buffer)
    }
}

// Multi-message decoder: returns NetworkPacket directly (application handles routing)
impl Decoder<NetworkPacket> for Pl3xusBincodeCodec {
    type Error = NetworkError;
    type Encoded = [u8];

    fn decode(val: &Self::Encoded) -> Result<NetworkPacket, Self::Error> {
        if val.len() < 8 {
            return Err(NetworkError::Serialization);
        }

        let length_bytes: [u8; 8] = val[..8]
            .try_into()
            .map_err(|_| NetworkError::Serialization)?;
        let _length = u64::from_le_bytes(length_bytes);

        // Decode directly to NetworkPacket
        // The application layer will handle unwrapping and routing
        bincode::serde::decode_from_slice(&val[8..], bincode::config::standard())
            .map(|(packet, _)| packet)
            .map_err(|_| NetworkError::Serialization)
    }
}

/// Convenience codec for single message-type connections.
///
/// This codec automatically wraps/unwraps NetworkPacket, making it convenient
/// for simple examples with dedicated connections per message type.
///
/// ## Usage
///
/// ```rust,ignore
/// use pl3xus_common::codec::Pl3xusBincodeSingleMsgCodec;
///
/// let ws = use_websocket_with_options::<UserChatMessage, NewChatMessage, Pl3xusBincodeSingleMsgCodec>(
///     "ws://127.0.0.1:8081",
///     options
/// );
/// ```
///
/// **Note:** For production applications with multiple message types, use `Pl3xusBincodeCodec` instead.
pub struct Pl3xusBincodeSingleMsgCodec;

// Single-message type encoder: wraps T in NetworkPacket
impl<T: Pl3xusMessage> Encoder<T> for Pl3xusBincodeSingleMsgCodec {
    type Error = NetworkError;
    type Encoded = Vec<u8>;

    fn encode(val: &T) -> Result<Self::Encoded, Self::Error> {
        // Wrap the message in NetworkPacket
        let packet = NetworkPacket {
            type_name: T::type_name().to_string(),
            schema_hash: T::schema_hash(),
            data: bincode::serde::encode_to_vec(val, bincode::config::standard())
                .map_err(|_| NetworkError::Serialization)?,
        };

        // Encode the NetworkPacket with bincode
        let encoded_packet = bincode::serde::encode_to_vec(&packet, bincode::config::standard())
            .map_err(|_| NetworkError::Serialization)?;

        // Prepend the 8-byte length prefix (REQUIRED for WebSocket protocol)
        let len = encoded_packet.len() as u64;
        let mut buffer = Vec::with_capacity(8 + encoded_packet.len());
        buffer.extend_from_slice(&len.to_le_bytes());
        buffer.extend_from_slice(&encoded_packet);

        Ok(buffer)
    }
}

// Single-message type decoder: unwraps NetworkPacket to get T
impl<T: DeserializeOwned> Decoder<T> for Pl3xusBincodeSingleMsgCodec {
    type Error = NetworkError;
    type Encoded = [u8];

    fn decode(val: &Self::Encoded) -> Result<T, Self::Error> {
        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::console;
            console::log_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] Received {} bytes", val.len()).into());
        }

        // Read the 8-byte length prefix
        if val.len() < 8 {
            #[cfg(target_arch = "wasm32")]
            {
                use web_sys::console;
                console::error_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] ERROR: Buffer too small ({}  bytes), need at least 8", val.len()).into());
            }
            return Err(NetworkError::Serialization);
        }

        let length_bytes: [u8; 8] = val[..8]
            .try_into()
            .map_err(|_| NetworkError::Serialization)?;
        let _length = u64::from_le_bytes(length_bytes);

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::console;
            console::log_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] Length prefix: {}", _length).into());
        }

        // Decode the NetworkPacket
        let packet: NetworkPacket = bincode::serde::decode_from_slice(&val[8..], bincode::config::standard())
            .map_err(|_e| {
                #[cfg(target_arch = "wasm32")]
                {
                    use web_sys::console;
                    console::error_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] ERROR: Failed to deserialize NetworkPacket: {:?}", _e).into());
                }
                NetworkError::Serialization
            })
            .map(|(packet, _)| packet)?;

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::console;
            console::log_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] NetworkPacket decoded: type_name={}, schema_hash={}, data_len={}", packet.type_name, packet.schema_hash, packet.data.len()).into());
        }

        // Decode the message from the packet's data
        bincode::serde::decode_from_slice(&packet.data, bincode::config::standard())
            .map_err(|_e| {
                #[cfg(target_arch = "wasm32")]
                {
                    use web_sys::console;
                    console::error_1(&format!("[Pl3xusBincodeSingleMsgCodec::decode] ERROR: Failed to deserialize message from packet data: {:?}", _e).into());
                }
                NetworkError::Serialization
            })
            .map(|(msg, _)| msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_message_codec() {
        // Test the multi-message codec (Pl3xusBincodeCodec)
        // This codec works with NetworkPacket directly

        let packet = NetworkPacket {
            type_name: "TestMessage".to_string(),
            schema_hash: 0x1234567890abcdef,
            data: vec![1, 2, 3, 4, 5],
        };

        // Test encoding
        let enc = Pl3xusBincodeCodec::encode(&packet).unwrap();

        // Should have 8-byte length prefix + encoded packet
        assert!(enc.len() > 8);

        // First 8 bytes should be the length
        let length_bytes: [u8; 8] = enc[..8].try_into().unwrap();
        let length = u64::from_le_bytes(length_bytes);
        assert_eq!(length as usize, enc.len() - 8);

        // Test decoding
        let dec: NetworkPacket = Pl3xusBincodeCodec::decode(&enc).unwrap();
        assert_eq!(dec.type_name, packet.type_name);
        assert_eq!(dec.schema_hash, packet.schema_hash);
        assert_eq!(dec.data, packet.data);
    }

    #[test]
    fn test_single_message_codec() {
        // Test the single-message codec (Pl3xusBincodeSingleMsgCodec)
        // This codec automatically wraps/unwraps NetworkPacket

        #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
        struct TestMessage {
            s: String,
            i: i32,
        }

        let msg = TestMessage {
            s: String::from("party time 🎉"),
            i: 42,
        };

        // Test encoding (wraps in NetworkPacket automatically)
        let enc = Pl3xusBincodeSingleMsgCodec::encode(&msg).unwrap();

        // Should have 8-byte length prefix + encoded NetworkPacket
        assert!(enc.len() > 8);

        // First 8 bytes should be the length
        let length_bytes: [u8; 8] = enc[..8].try_into().unwrap();
        let length = u64::from_le_bytes(length_bytes);
        assert_eq!(length as usize, enc.len() - 8);

        // Test decoding (unwraps NetworkPacket automatically)
        let dec: TestMessage = Pl3xusBincodeSingleMsgCodec::decode(&enc).unwrap();
        assert_eq!(dec, msg);
    }
}
