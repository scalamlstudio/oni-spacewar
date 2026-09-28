//! Matchbox <-> GGRS glue plus a tiny network emulator (from the TAKOAI-18 spike).
//!
//! `matchbox_socket` 0.14 ships a GGRS adapter, but it is built against ggrs
//! 0.11 while `bevy_ggrs` 0.22 needs ggrs 0.13, so we implement
//! `ggrs::NonBlockingSocket` ourselves (same wire format: bincode + serde).
//!
//! The emulator sits inside that adapter: every outgoing packet is dropped with
//! probability `loss`, otherwise held for `delay ± jitter` before it is handed to
//! the WebRTC channel. Applied on both peers, so the added round trip is
//! `2 * delay`. This lives entirely outside the rollback simulation, so using
//! the wall clock and an unseeded RNG here is fine.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use ggrs::{Message, NonBlockingSocket};
use matchbox_socket::{Packet, PeerId, WebRtcChannel, WebRtcSocket};

#[derive(Clone, Copy, Debug, Default)]
pub struct NetEmuConfig {
    /// One-way added delay applied to every outgoing packet.
    pub delay: Duration,
    /// Uniform jitter (+/-) added to `delay`.
    pub jitter: Duration,
    /// Probability in `0.0..=1.0` that an outgoing packet is dropped.
    pub loss: f64,
}

#[derive(Default, Clone, Copy, Debug)]
pub struct NetEmuStats {
    pub sent: u64,
    pub dropped: u64,
}

pub struct EmulatedGgrsSocket {
    channel: WebRtcChannel,
    emu: NetEmuConfig,
    queue: VecDeque<(Instant, PeerId, Packet)>,
    rng: u64,
    pub stats: NetEmuStats,
}

impl EmulatedGgrsSocket {
    pub fn new(channel: WebRtcChannel, emu: NetEmuConfig) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let seed = (std::process::id() as u64) << 32 ^ now.as_nanos() as u64;
        Self {
            channel,
            emu,
            queue: VecDeque::new(),
            rng: seed | 1,
            stats: NetEmuStats::default(),
        }
    }

    fn rand_f64(&mut self) -> f64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn flush_due(&mut self) {
        let now = Instant::now();
        // Packets can become due out of order when jitter is on; scan everything.
        let mut i = 0;
        while i < self.queue.len() {
            if self.queue[i].0 <= now {
                let (_, peer, packet) = self.queue.remove(i).unwrap();
                self.channel.send(packet, peer);
            } else {
                i += 1;
            }
        }
    }
}

impl NonBlockingSocket<PeerId> for EmulatedGgrsSocket {
    fn send_to(&mut self, msg: &Message, addr: &PeerId) {
        self.stats.sent += 1;
        if self.emu.loss > 0.0 && self.rand_f64() < self.emu.loss {
            self.stats.dropped += 1;
            return;
        }
        let packet: Packet = bincode::serde::encode_to_vec(msg, bincode::config::standard())
            .expect("failed to serialize ggrs packet")
            .into_boxed_slice();
        if self.emu.delay.is_zero() && self.emu.jitter.is_zero() {
            self.channel.send(packet, *addr);
            return;
        }
        let jitter = self.emu.jitter.as_secs_f64() * (self.rand_f64() * 2.0 - 1.0);
        let delay = (self.emu.delay.as_secs_f64() + jitter).max(0.0);
        self.queue.push_back((
            Instant::now() + Duration::from_secs_f64(delay),
            *addr,
            packet,
        ));
        self.flush_due();
    }

    fn receive_all_messages(&mut self) -> Vec<(PeerId, Message)> {
        // Called every Bevy frame by bevy_ggrs (poll_remote_clients), so this is
        // also where delayed packets get released.
        self.flush_due();
        self.channel
            .receive()
            .into_iter()
            .filter_map(|(peer, packet)| {
                bincode::serde::decode_from_slice(&packet, bincode::config::standard())
                    .ok()
                    .map(|(msg, _)| (peer, msg))
            })
            .collect()
    }
}

impl Drop for EmulatedGgrsSocket {
    fn drop(&mut self) {
        println!(
            "net-emu: sent {} packets, dropped {} ({:.2}%)",
            self.stats.sent,
            self.stats.dropped,
            100.0 * self.stats.dropped as f64 / self.stats.sent.max(1) as f64
        );
    }
}

/// Player list in a consistent order on every peer (sorted peer ids).
/// Mirrors `WebRtcSocket::players` from matchbox's `ggrs` feature, which we
/// can't enable because it pulls in ggrs 0.11.
pub fn ggrs_players(socket: &mut WebRtcSocket) -> Option<Vec<ggrs::PlayerType<PeerId>>> {
    let our_id = socket.id()?;
    let mut ids: Vec<_> = socket
        .connected_peers()
        .chain(std::iter::once(our_id))
        .collect();
    ids.sort();
    Some(
        ids.into_iter()
            .map(|id| {
                if id == our_id {
                    ggrs::PlayerType::Local
                } else {
                    ggrs::PlayerType::Remote(id)
                }
            })
            .collect(),
    )
}
