use fvm_shared::METHOD_CONSTRUCTOR;

define_singleton!(CRON_ACTOR = 3);

/// Cron actor methods available
#[repr(u64)]
pub enum Method {
    Constructor = METHOD_CONSTRUCTOR,
    EpochTick = 2,
}
