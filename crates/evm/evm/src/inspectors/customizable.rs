use alloy_primitives::{Address, Log, U256};
use foundry_evm_core::backend::DatabaseError;
use revm::{
    interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome, Interpreter},
    primitives::{EVMError, Env},
    Database, EvmContext, Inspector,
};
use std::{
    any::Any,
    fmt::{Debug, Formatter},
};

pub struct Customizable {
    pub inspector: Box<dyn CustomizableInspector>,
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

pub struct InnerEvmContextWrap<'a, 'b> {
    pub env: &'b mut Box<Env>,
    pub journaled_state: &'a mut revm::JournaledState,
    pub db: &'a mut (dyn Database<Error = DatabaseError> + 'b),
    pub error: &'b mut Result<(), EVMError<DatabaseError>>,
    #[cfg(feature = "optimism")]
    pub l1_block_info: &'b mut Option<crate::optimism::L1BlockInfo>,
}

// pub struct EvmContextWrap<'a, 'b: 'a> {
//     /// Inner EVM context.
//     pub inner: InnerEvmContextWrap<'a, 'b>,
// }

pub trait CustomizableInspector: Any + Send + Sync {
    fn as_any(&self) -> &dyn Any;

    fn into_box_any(self: Box<Self>) -> Box<dyn Any>;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn clone_box(&self) -> Box<dyn CustomizableInspector>;

    /// Called before the interpreter is initialized.
    ///
    /// If `interp.instruction_result` is set to anything other than
    /// [crate::interpreter::InstructionResult::Continue] then the execution of the interpreter
    /// is skipped.
    #[inline]
    fn initialize_interp(
        &mut self,
        _interp: &mut Interpreter,
        _context: InnerEvmContextWrap<'_, '_>,
    ) {
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
    fn step(&mut self, _interp: &mut Interpreter, _context: InnerEvmContextWrap<'_, '_>) {}

    /// Called after `step` when the instruction has been executed.
    ///
    /// Setting `interp.instruction_result` to anything other than
    /// [crate::interpreter::InstructionResult::Continue] alters the execution
    /// of the interpreter.
    #[inline]
    fn step_end(&mut self, _interp: &mut Interpreter, _context: InnerEvmContextWrap<'_, '_>) {}

    /// Called when a log is emitted.
    #[inline]
    fn log(&mut self, _context: InnerEvmContextWrap<'_, '_>, _log: &Log) {}

    /// Called whenever a call to a contract is about to start.
    ///
    /// InstructionResulting anything other than [crate::interpreter::InstructionResult::Continue]
    /// overrides the result of the call.
    #[inline]
    fn call(&mut self, _context: InnerEvmContextWrap<'_, '_>, _inputs: &mut CallInputs) {}

    /// Called when a call to a contract has concluded.
    ///
    /// The returned [CallOutcome] is used as the result of the call.
    ///
    /// This allows the inspector to modify the given `result` before returning it.
    #[inline]
    fn call_end(
        &mut self,
        _context: InnerEvmContextWrap<'_, '_>,
        _inputs: &CallInputs,
        _outcome: &CallOutcome,
    ) {
    }

    /// Called when a contract is about to be created.
    ///
    /// If this returns `Some` then the [CreateOutcome] is used to override the result of the
    /// creation.
    ///
    /// If this returns `None` then the creation proceeds as normal.
    #[inline]
    fn create(&mut self, _context: InnerEvmContextWrap<'_, '_>, _inputs: &mut CreateInputs) {}

    /// Called when a contract has been created.
    ///
    /// InstructionResulting anything other than the values passed to this function (`(ret,
    /// remaining_gas, address, out)`) will alter the result of the create.
    #[inline]
    fn create_end(
        &mut self,
        _context: InnerEvmContextWrap<'_, '_>,
        _inputs: &CreateInputs,
        _outcome: &CreateOutcome,
    ) {
    }

    /// Called when a contract has been self-destructed with funds transferred to target.
    #[inline]
    fn selfdestruct(&mut self, _contract: Address, _target: Address, _value: U256) {}
}

impl Customizable {
    pub fn new<T: CustomizableInspector + Sync + 'static>(inspector: T) -> Self {
        Customizable { inspector: Box::new(inspector) }
    }

    pub fn get_inspector<T: CustomizableInspector + Sync + 'static>(&self) -> Option<&T> {
        let df = self.inspector.as_any().downcast_ref::<T>();
        df
    }

    pub fn take_inspector<T: CustomizableInspector + Sync + 'static>(self) -> Option<T> {
        let inspector = self.inspector.into_box_any().downcast::<T>();
        match inspector {
            Ok(d) => Some(*d),
            Err(e) => None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DefaultInspector {}

impl CustomizableInspector for DefaultInspector {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_box_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn CustomizableInspector> {
        Box::new(self.clone())
    }
}

impl Default for Customizable {
    fn default() -> Self {
        Customizable { inspector: Box::new(DefaultInspector::default()) }
    }
}

impl<DB: Database<Error = DatabaseError>> Inspector<DB> for Customizable {
    fn initialize_interp(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };

        self.inspector.initialize_interp(interp, evm_context);
    }

    fn step(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.step(interp, evm_context)
    }

    fn step_end(&mut self, interp: &mut Interpreter, context: &mut EvmContext<DB>) {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.step_end(interp, evm_context)
    }

    fn log(&mut self, context: &mut EvmContext<DB>, log: &Log) {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.log(evm_context, log)
    }

    fn call(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CallInputs,
    ) -> Option<CallOutcome> {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.call(evm_context, inputs);

        None
    }

    fn call_end(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &CallInputs,
        outcome: CallOutcome,
    ) -> CallOutcome {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.call_end(evm_context, inputs, &outcome);
        outcome
    }

    fn create(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &mut CreateInputs,
    ) -> Option<CreateOutcome> {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.create(evm_context, inputs);

        None
    }

    fn create_end(
        &mut self,
        context: &mut EvmContext<DB>,
        inputs: &CreateInputs,
        outcome: CreateOutcome,
    ) -> CreateOutcome {
        let evm_context = InnerEvmContextWrap {
            env: &mut context.inner.env,
            journaled_state: &mut context.inner.journaled_state,
            db: &mut context.inner.db as &mut (dyn Database<Error = DatabaseError>),
            error: &mut context.inner.error,
            #[cfg(feature = "optimism")]
            l1_block_info: &mut data.l1_block_info,
        };
        self.inspector.create_end(evm_context, inputs, &outcome);
        outcome
    }

    fn selfdestruct(&mut self, contract: Address, target: Address, value: U256) {
        self.inspector.selfdestruct(contract, target, value);
    }
}
