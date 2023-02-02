use async_trait::async_trait;

mod externs;
mod vm;

/// The interpreter applies messages on some state.
///
/// It is asynchronous so that message execution can have side effects,
/// such as scheduling the resolution of embedded CIDs. These kind of
/// side effects could be signalled through the `Output` as well, but
/// the intention is to be able to stack interpreters, and at least
/// some of them might want execute something asynchronous.
///
/// There is no separate type for `Error`, only `Output`. The reason
/// is that we'll be calling high level executors internally that
/// already have their internal error handling, returning all domain
/// errors such as `OutOfGas` in their output, and only using the
/// error case for things that are independent of the message itself.
#[async_trait]
trait Interpreter: Sync + Send {
    type State: Send;
    type Message: Send;
    type Output;

    /// Apply a message onto the state.
    ///
    /// The state is taken by value, so there's no issue with sharing
    /// mutable references in futures. The modified value should be
    /// returned along with the return value.
    ///
    /// Only return an error case if something truly unexpected happens
    /// that should stop message processing altogether; otherwise use
    /// the output for signalling all execution results.
    async fn exec_msg(
        &self,
        state: Self::State,
        msg: Self::Message,
    ) -> anyhow::Result<(Self::State, Self::Output)>;
}
