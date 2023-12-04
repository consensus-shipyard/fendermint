// Copyright 2021-2023 Protocol Labs
// SPDX-License-Identifier: Apache-2.0, MIT

use fvm::call_manager::CallManager;
use fvm::gas::Gas;
use fvm::gas::GasTimer;
use fvm::gas::PriceList;
use fvm::kernel::*;
use fvm::syscalls::bind::BindSyscall;
use fvm::syscalls::InvocationData;
use fvm::{
    ambassador_impl_ActorOps, ambassador_impl_CircSupplyOps, ambassador_impl_CryptoOps,
    ambassador_impl_DebugOps, ambassador_impl_EventOps, ambassador_impl_GasOps,
    ambassador_impl_IpldBlockOps, ambassador_impl_LimiterOps, ambassador_impl_MessageOps,
    ambassador_impl_NetworkOps, ambassador_impl_RandomnessOps, ambassador_impl_SelfOps,
    DefaultKernel,
};

use fvm_shared::clock::ChainEpoch;
use fvm_shared::crypto::signature::*;
use fvm_shared::randomness::RANDOMNESS_LENGTH;
use fvm_shared::sys::out::network::NetworkContext;
use fvm_shared::sys::out::vm::MessageContext;
use fvm_shared::{address::Address, econ::TokenAmount, sys::SendFlags, ActorID, MethodNum};

use cid::Cid;
use multihash::MultihashGeneric;

use ambassador::Delegate;
use wasmtime::Linker;

// define the custom kernel syscall here
pub trait CustomKernel: Kernel {
    fn my_custom_syscall(&self) -> Result<()>;
}

#[derive(Delegate)]
#[delegate(IpldBlockOps)]
#[delegate(ActorOps)]
#[delegate(CircSupplyOps)]
#[delegate(CryptoOps)]
#[delegate(DebugOps)]
#[delegate(EventOps)]
#[delegate(GasOps)]
#[delegate(MessageOps)]
#[delegate(NetworkOps)]
#[delegate(RandomnessOps)]
#[delegate(SelfOps)]
#[delegate(LimiterOps)]
// we define the implementation of the custom kernel here. We use ambassador to delegate most of the existing syscalls to the default kernel
pub struct DefaultCustomKernel<K>(pub K)
where
    K: Kernel;

// we need to implement the custom kernel trait for the default custom kernel
impl<C> CustomKernel for DefaultCustomKernel<DefaultKernel<C>>
where
    C: CallManager,
    DefaultCustomKernel<DefaultKernel<C>>: Kernel,
{
    fn my_custom_syscall(&self) -> Result<()> {
        // TODO: Implement the kernel syscall here.
        Ok(())
    }
}

// we need to implement the kernel trait for the default custom kernel, here we simply delegate to the default kernel
impl<C> Kernel for DefaultCustomKernel<DefaultKernel<C>>
where
    C: CallManager,
{
    type CallManager = C;

    fn into_inner(self) -> (Self::CallManager, BlockRegistry)
    where
        Self: Sized,
    {
        self.0.into_inner()
    }

    fn machine(&self) -> &<Self::CallManager as CallManager>::Machine {
        self.0.machine()
    }

    fn send<K: Kernel<CallManager = C>>(
        &mut self,
        recipient: &Address,
        method: u64,
        params: BlockId,
        value: &TokenAmount,
        gas_limit: Option<Gas>,
        flags: SendFlags,
    ) -> Result<CallResult> {
        self.0
            .send::<Self>(recipient, method, params, value, gas_limit, flags)
    }

    fn upgrade_actor<K: Kernel<CallManager = Self::CallManager>>(
        &mut self,
        new_code_cid: Cid,
        params_id: BlockId,
    ) -> Result<CallResult> {
        self.0.upgrade_actor::<Self>(new_code_cid, params_id)
    }

    fn new(
        mgr: C,
        blocks: BlockRegistry,
        caller: ActorID,
        actor_id: ActorID,
        method: MethodNum,
        value_received: TokenAmount,
        read_only: bool,
    ) -> Self {
        DefaultCustomKernel(DefaultKernel::new(
            mgr,
            blocks,
            caller,
            actor_id,
            method,
            value_received,
            read_only,
        ))
    }
}

// we need to implement the syscall handler for the default custom kernel which binds our syscalls to the wasmtime linker
impl<C> SyscallHandler<DefaultCustomKernel<DefaultKernel<C>>>
    for DefaultCustomKernel<DefaultKernel<C>>
where
    C: CallManager,
{
    fn bind_syscalls(
        &self,
        linker: &mut Linker<InvocationData<DefaultCustomKernel<DefaultKernel<C>>>>,
    ) -> anyhow::Result<()> {
        self.0.bind_syscalls(linker)?;

        // Now bind our custom syscalls
        linker.bind("my_custom_kernel", "my_custom_syscall", my_custom_syscall)?;

        Ok(())
    }
}

// this is the actual syscall implementation as registered in bind_syscalls
pub fn my_custom_syscall(context: fvm::syscalls::Context<'_, impl CustomKernel>) -> Result<()> {
    context.kernel.my_custom_syscall()?;
    Ok(())
}

pub mod sdk {
    // This is the direct sdk call the wasm actor calls
    pub fn my_custom_syscall() {
        unsafe {
            sys::my_custom_syscall().unwrap();
        }
    }

    pub mod sys {
        use fvm_sdk::fvm_syscalls;

        fvm_syscalls! {
            module = "my_custom_kernel";
            pub fn my_custom_syscall() -> Result<()>;
        }
    }
}
