//! Replay a command stream through the parser and watchdog — including a
//! link dropout — and watch the safety behaviour.
//!
//! Run with: `cargo run --example cmdvel_replay`

#[allow(dead_code, unused_imports)]
#[path = "../src/comms/cmdvel.rs"]
mod cmdvel;

use cmdvel::{encode_frame, parse_frame, DriveCommand, Watchdog, ENCODE_MAX};

fn main() {
    let mut dog = Watchdog::new(500);

    // (time_ms, frame arriving at that moment — None = radio silence)
    let timeline: &[(u32, Option<&[u8]>)] = &[
        (0, Some(b"300,300")),      // forward
        (200, Some(b"500,-500\n")), // spin
        (400, Some(b"oops")),       // corrupt frame — ignored, old cmd holds
        (600, None),                // silence…
        (800, None),                // …watchdog will trip at 900 (400+500)
        (1000, Some(b"200,200")),   // link recovers
    ];

    println!("t(ms)  frame          -> motors run");
    for &(t, frame) in timeline {
        let note = match frame {
            Some(bytes) => match parse_frame(bytes) {
                Ok(cmd) => {
                    dog.feed(cmd, t);
                    format!("{:?}", String::from_utf8_lossy(bytes))
                }
                Err(err) => format!("{:?} (rejected: {err})", String::from_utf8_lossy(bytes)),
            },
            None => "-".to_string(),
        };
        let run = dog.command(t);
        println!("{t:5}  {note:22} -> left {:5}  right {:5}", run.left, run.right);
    }

    // The controller side of the link uses the encoder:
    let mut buf = [0u8; ENCODE_MAX];
    let len = encode_frame(DriveCommand { left: -750, right: 750 }, &mut buf)
        .expect("ENCODE_MAX-sized buffer");
    println!(
        "\nencoded for sending: {:?}",
        core::str::from_utf8(&buf[..len]).expect("encoder emits ASCII")
    );
}
