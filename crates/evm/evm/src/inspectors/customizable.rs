use alloy_primitives::{Address, Bytes, B256, U256};
use foundry_evm_core::backend::DatabaseError;
use revm::{
    interpreter::{CallInputs, CreateInputs, Gas, InstructionResult, Interpreter},
    Database, EVMData, Inspector,
};
use std::{
    any::Any,
    fmt::{Debug, Formatter},
};

pub struct Customizable {
    pub inspector: Box<dyn CustomizableInspector + Sync + 'static>,
}

impl Clone for Customizable {
    fn clone(&self) -> Self {
        Customizable { inspector: self.inspector.clone_box() }
    }
}

impl Debug for Customizable {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Customizable").finish()
    }
}

pub struct EVMDataWrap<'a, 'b: 'a> {
    pub env: &'a &'b mut revm::primitives::Env,
    pub journaled_state: &'a mut revm::JournaledState,
    pub db: &'a mut (dyn Database<Error = DatabaseError> + 'b),
    pub error: &'b mut Option<DatabaseError>,
    pub precompiles: &'b mut revm::precompile::Precompiles,
    #[cfg(feature = "optimism")]
    pub l1_block_info: &'b mut Option<crate::optimism::L1BlockInfo>,
}

pub trait CustomizableInspector: Any + Sync {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn clone_box(&self) -> Box<dyn CustomizableInspector + Sync>;

    /// Called before the interpreter is initialized.
    ///
    /// If `interp.instruction_result` is set to anything other than [InstructionResult::Continue]
    /// then the execution of the interpreter is skipped.
    #[inline]
    fn initialize_interp(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMDataWrap<'_, '_>) {
        let _ = interp;
        let _ = data;
    }

    /// Called on each step of the interpreter.
    ///
    /// Information about the current execution, including the memory, stack and more is available
    /// on `interp` (see [Interpreter]).
    ///
    /// # Example
    ///
    /// To get the current opcode, use `interp.current_opcode()`.
    #[inline]
    fn step(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMDataWrap<'_, '_>) {
        let _ = interp;
        let _ = data;
    }

    /// Called when a log is emitted.
    #[inline]
    fn log(
        &mut self,
        evm_data: &mut EVMDataWrap<'_, '_>,
        address: &Address,
        topics: &[B256],
        data: &Bytes,
    ) {
        let _ = evm_data;
        let _ = address;
        let _ = topics;
        let _ = data;
    }

    /// Called after `step` when the instruction has been executed.
    ///
    /// Setting `interp.instruction_result` to anything other than [InstructionResult::Continue]
    /// alters the execution of the interpreter.
    #[inline]
    fn step_end(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMDataWrap<'_, '_>) {
        let _ = interp;
        let _ = data;
    }

    /// Called whenever a call to a contract is about to start.
    ///
    /// InstructionResulting anything other than [InstructionResult::Continue] overrides the result
    /// of the call.
    #[inline]
    fn call(
        &mut self,
        data: &mut EVMDataWrap<'_, '_>,
        inputs: &mut CallInputs,
    ) -> (InstructionResult, Gas, Bytes) {
        let _ = data;
        let _ = inputs;
        (InstructionResult::Continue, Gas::new(0), Bytes::new())
    }

    /// Called when a call to a contract has concluded.
    ///
    /// InstructionResulting anything other than the values passed to this function (`(ret,
    /// remaining_gas, out)`) will alter the result of the call.
    #[inline]
    fn call_end(
        &mut self,
        data: &mut EVMDataWrap<'_, '_>,
        inputs: &CallInputs,
        remaining_gas: Gas,
        ret: InstructionResult,
        out: Bytes,
    ) -> (InstructionResult, Gas, Bytes) {
        let _ = data;
        let _ = inputs;
        (ret, remaining_gas, out)
    }

    /// Called when a contract is about to be created.
    ///
    /// InstructionResulting anything other than [InstructionResult::Continue] overrides the result
    /// of the creation.
    #[inline]
    fn create(
        &mut self,
        data: &mut EVMDataWrap<'_, '_>,
        inputs: &mut CreateInputs,
    ) -> (InstructionResult, Option<Address>, Gas, Bytes) {
        let _ = data;
        let _ = inputs;
        (InstructionResult::Continue, None, Gas::new(0), Bytes::default())
    }

    /// Called when a contract has been created.
    ///
    /// InstructionResulting anything other than the values passed to this function (`(ret,
    /// remaining_gas, address, out)`) will alter the result of the create.
    #[inline]
    fn create_end(
        &mut self,
        data: &mut EVMDataWrap<'_, '_>,
        inputs: &CreateInputs,
        ret: InstructionResult,
        address: Option<Address>,
        remaining_gas: Gas,
        out: Bytes,
    ) -> (InstructionResult, Option<Address>, Gas, Bytes) {
        let _ = data;
        let _ = inputs;
        (ret, address, remaining_gas, out)
    }

    /// Called when a contract has been self-destructed with funds transferred to target.
    #[inline]
    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        let _ = contract;
        let _ = target;
        let _ = value;
    }
}

impl Customizable {
    pub fn new<T: CustomizableInspector + Sync + 'static>(inspector: T) -> Self {
        Customizable { inspector: Box::new(inspector) }
    }

    pub fn get_inspector<T: CustomizableInspector + Sync + 'static>(&self) -> Option<&T> {
        let df = self.inspector.as_any().downcast_ref::<T>();
        df
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefaultInspector {}

impl CustomizableInspector for DefaultInspector {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn CustomizableInspector + Sync> {
        Box::new(self.clone())
    }
}

impl Default for Customizable {
    fn default() -> Self {
        Customizable { inspector: Box::new(DefaultInspector::default()) }
    }
}

impl<DB: Database<Error = DatabaseError>> Inspector<DB> for Customizable {
    fn initialize_interp(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMData<'_, DB>) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.initialize_interp(interp, &mut wrap);
    }

    fn step(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMData<'_, DB>) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.step(interp, &mut wrap);
    }

    fn log(
        &mut self,
        evm_data: &mut EVMData<'_, DB>,
        address: &Address,
        topics: &[B256],
        data: &Bytes,
    ) {
        let mut wrap = EVMDataWrap {
            env: &mut evm_data.env,
            journaled_state: &mut evm_data.journaled_state,
            db: evm_data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut evm_data.error,
            precompiles: &mut evm_data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut evm_data.l1_block_info,
        };
        self.inspector.log(&mut wrap, address, topics, data);
    }

    fn step_end(&mut self, interp: &mut Interpreter<'_>, data: &mut EVMData<'_, DB>) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.step_end(interp, &mut wrap);
    }

    fn call(
        &mut self,
        data: &mut EVMData<'_, DB>,
        inputs: &mut CallInputs,
    ) -> (InstructionResult, Gas, Bytes) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.call(&mut wrap, inputs)
    }

    fn call_end(
        &mut self,
        data: &mut EVMData<'_, DB>,
        inputs: &CallInputs,
        remaining_gas: Gas,
        ret: InstructionResult,
        out: Bytes,
    ) -> (InstructionResult, Gas, Bytes) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.call_end(&mut wrap, inputs, remaining_gas, ret, out.clone())
    }

    fn create(
        &mut self,
        data: &mut EVMData<'_, DB>,
        inputs: &mut CreateInputs,
    ) -> (InstructionResult, Option<Address>, Gas, Bytes) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.create(&mut wrap, inputs)
    }

    fn create_end(
        &mut self,
        data: &mut EVMData<'_, DB>,
        inputs: &CreateInputs,
        ret: InstructionResult,
        address: Option<Address>,
        remaining_gas: Gas,
        out: Bytes,
    ) -> (InstructionResult, Option<Address>, Gas, Bytes) {
        let mut wrap = EVMDataWrap {
            env: &mut data.env,
            journaled_state: &mut data.journaled_state,
            db: data.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut data.error,
            precompiles: &mut data.precompiles,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.create_end(&mut wrap, inputs, ret, address, remaining_gas, out.clone())
    }

    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        self.inspector.selfdestruct(contract, target, value)
    }
}
