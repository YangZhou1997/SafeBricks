use failure::Error;
use native::mbuf::MBuf;
use packets::Packet;
use std::collections::HashMap;
use interface::PacketTx;
pub use self::emit_batch::*;
pub use self::filter_batch::*;
pub use self::filtermap_batch::*;
pub use self::foreach_batch::*;
pub use self::groupby_batch::*;
pub use self::map_batch::*;
pub use self::queue_batch::*;
pub use self::receive_batch::*;
pub use self::send_batch::*;
pub use self::sendall_batch::*;

mod emit_batch;
mod filter_batch;
mod filtermap_batch;
mod foreach_batch;
mod groupby_batch;
mod map_batch;
mod queue_batch;
mod receive_batch;
mod send_batch;
mod sendall_batch;

/// Error when processing packets
#[derive(Debug)]
pub enum PacketError {
    /// Processing is complete; emit the packet
    Emit(*mut MBuf),
    /// The packet is intentionally dropped
    Drop(*mut MBuf),
    /// The packet is aborted due to an error
    Abort(*mut MBuf, Error),
}

/// Common behavior for a batch of packets
pub trait Batch {
    /// The packet type
    type Item: Packet;

    /// Returns the next packet in the batch
    fn next(&mut self) -> Option<Result<Self::Item, PacketError>>;

    /// Receives a new batch
    fn receive(&mut self);

    /// Appends a filter operator to the end of the pipeline
    #[inline]
    fn filter<P>(self, predicate: P) -> FilterBatch<Self, P>
    where
        P: FnMut(&Self::Item) -> bool,
        Self: Sized,
    {
        FilterBatch::new(self, predicate)
    }

    ///
    #[inline]
    fn filter_map<T: Packet, F>(self, f: F) -> FilterMapBatch<Self, T, F>
    where
        F: FnMut(Self::Item) -> Result<Option<T>, Error>,
        Self: Sized,
    {
        FilterMapBatch::new(self, f)
    }

    /// Appends a map operator to the end of the pipeline
    #[inline]
    fn map<T: Packet, M>(self, map: M) -> MapBatch<Self, T, M>
    where
        M: FnMut(Self::Item) -> Result<T, Error>,
        Self: Sized,
    {
        MapBatch::new(self, map)
    }

    /// Appends a for_each operator to the end of the pipeline
    ///
    /// Use for side-effects on packets, meaning the packets will not be
    /// transformed byte-wise.
    #[inline]
    fn for_each<F>(self, fun: F) -> ForEachBatch<Self, F>
    where
        F: FnMut(&Self::Item) -> Result<(), Error>,
        Self: Sized,
    {
        ForEachBatch::new(self, fun)
    }

    /// Appends a group_by operator to the end of the pipeline
    ///
    /// * `selector` - a function that receives a reference to `B::Item` and
    /// evaluates to a discriminator value. The source batch will be split
    /// into subgroups based on this value.
    ///
    /// * `composer` - a function that composes the pipelines for the subgroups
    /// based on the discriminator values.
    ///
    /// # Example
    ///
    /// ```
    /// let batch = batch.group_by(
    ///     |packet| packet.protocol(),
    ///     |groups| {
    ///         compose!(
    ///             groups,
    ///             ProtocolNumbers::Tcp => |group| {
    ///                 group.map(handle_tcp)
    ///             },
    ///             ProtocolNumbers::Udp => |group| {
    ///                 group.map(handle_udp)
    ///             }
    ///         )
    ///     }
    /// );
    /// ```
    #[inline]
    fn group_by<K, S, C>(self, selector: S, composer: C) -> GroupByBatch<Self, K, S>
    where
        K: Eq + Clone + std::hash::Hash,
        S: FnMut(&Self::Item) -> K,
        C: FnOnce(&mut HashMap<Option<K>, Box<PipelineBuilder<Self::Item>>>) -> (),
        Self: Sized,
    {
        GroupByBatch::new(self, selector, composer)
    }

    /// Appends a emit operator to the end of the pipeline
    ///
    /// Use when processing is complete and no further modifications are necessary.
    /// Any further operators will have no effect on packets that have been through
    /// the emit operator. Emit the packet as-is.
    fn emit(self) -> EmitBatch<Self>
    where
        Self: Sized,
    {
        EmitBatch::new(self)
    }

    /// Appends a send operator to the end of the pipeline
    ///
    /// Send marks the end of the pipeline. No more operators can be
    /// appended after send.
    #[inline]
    fn send<Tx: PacketTx>(self, port: Tx) -> SendBatch<Self, Tx>
    where
        Self: Sized,
    {
        SendBatch::new(self, port)
    }

    /// Appends a sendall operator to the end of the pipeline
    /// Send all packets not matter whether it has been set to drop in the previous Filter operators.
    ///
    /// Sendall marks the end of the pipeline. No more operators can be
    /// appended after send.
    #[inline]
    fn sendall<Tx: PacketTx>(self, port: Tx) -> SendAllBatch<Self, Tx>
    where
        Self: Sized,
    {
        SendAllBatch::new(self, port)
    }
}
