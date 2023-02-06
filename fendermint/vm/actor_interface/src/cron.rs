use fvm_shared::{ActorID, METHOD_CONSTRUCTOR};

/// Cron actor address.
pub const CRON_ACTOR_ID: ActorID = 3;

/// Cron actor methods available
#[repr(u64)]
pub enum Method {
    Constructor = METHOD_CONSTRUCTOR,
    EpochTick = 2,
}
