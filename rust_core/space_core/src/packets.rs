// space/rust_core/space_core/src/packets.rs

use pnet_packet::ethernet::EthernetPacket;
use tracing::debug;

pub fn process_packet_data(data: &[u8]) {
    debug!("Received packet data, length: {}", data.len());

    if let Some(ethernet_packet) = EthernetPacket::new(data) {
        debug!("Ethernet packet received from source: {:?}", ethernet_packet.get_source());
    } else {
        debug!("Could not parse Ethernet packet from data.");
    }
}