use fvm_shared::{econ::TokenAmount, message::Message};

// Copy of https://github.com/filecoin-project/ref-fvm/blob/fvm%40v3.3.1/fvm/src/gas/outputs.rs
mod output;

// https://github.com/filecoin-project/lotus/blob/6cc506f5cf751215be6badc94a960251c6453202/node/impl/full/eth.go#L2220C41-L2228
pub fn effective_gas_price(msg: &Message, gas_used: i64, base_fee: &TokenAmount) -> TokenAmount {
    let out = output::GasOutputs::compute(
        gas_used.try_into().expect("gas should be u64 convertible"),
        msg.gas_limit,
        base_fee,
        &msg.gas_fee_cap,
        &msg.gas_premium,
    );

    let total_spend = out.base_fee_burn + out.miner_tip + out.over_estimation_burn;

    if gas_used > 0 {
        TokenAmount::from_atto(total_spend.atto() / TokenAmount::from_atto(gas_used).atto())
    } else {
        TokenAmount::from_atto(0)
    }
}
