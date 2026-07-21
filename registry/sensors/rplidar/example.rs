//! Decode a captured RPLIDAR byte stream — no hardware needed.
//!
//! The protocol module is pure (`bytes in → typed values out`), so this
//! example replays a hand-built capture of a scan start: the descriptor,
//! then a handful of 5-byte measurement nodes. Feed the same functions from
//! a serial port to talk to a real device — see the wiring table in the
//! component README.
//!
//! Run with: `cargo run --example rplidar_decode`

// The component is vendored source, not a crate: include it by path so this
// example works even in a binary-only project.
#[allow(dead_code, unused_imports)]
#[path = "../src/sensors/rplidar/mod.rs"]
mod rplidar;

use rplidar::descriptor::{parse_descriptor, DESCRIPTOR_LEN};
use rplidar::scan_node::{parse_scan_node, SCAN_NODE_LEN};
use rplidar::Command;

fn main() {
    // ---- request: what we would write to the serial port -------------------
    let mut buf = [0u8; rplidar::MAX_REQUEST_LEN];
    let len = Command::Scan.encode(&mut buf);
    println!("SCAN request bytes:      {:02X?}", &buf[..len]);

    // ---- response: descriptor announcing an endless node stream ------------
    let capture: &[u8] = &[
        0xA5, 0x5A, 0x05, 0x00, 0x00, 0x40, 0x81, // descriptor: 5-byte nodes, multi
        0x3D, 0x81, 0x00, 0xE8, 0x03, // node: start, q=15, 1.0°, 250 mm
        0x3E, 0x01, 0x02, 0xD0, 0x07, // node: q=15, 4.0°, 500 mm
        0x3E, 0x81, 0x03, 0xB8, 0x0B, // node: q=15, 7.0°, 750 mm
        0x3E, 0x01, 0x05, 0x00, 0x00, // node: q=15, 10.0°, no return
    ];

    let (head, rest) = capture.split_at(DESCRIPTOR_LEN);
    let descriptor = parse_descriptor(head.try_into().expect("7 bytes"))
        .expect("valid descriptor");
    println!(
        "descriptor:              {} bytes per response, {:?}, data type {:#04x}",
        descriptor.len, descriptor.send_mode, descriptor.data_type
    );

    for chunk in rest.chunks_exact(SCAN_NODE_LEN) {
        let node = parse_scan_node(chunk.try_into().expect("5 bytes")).expect("valid node");
        println!(
            "  {}{:6.2}°  {:7.2} mm  quality {:2}{}",
            if node.start_flag { "▶ " } else { "  " },
            node.angle_deg(),
            node.distance_mm(),
            node.quality,
            if node.is_valid_measurement() { "" } else { "  (no return)" },
        );
    }
}
