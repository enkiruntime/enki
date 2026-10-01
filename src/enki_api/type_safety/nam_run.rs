use super::arg_match::GpuTypeMatch;
use crate::enki_api::context::ambient::with_active_flow_at;
use crate::enki_api::space::Space;
use anu::nam_args_api::{DispatchSafetyMode, NamDispatchMap, SpaceContract};

macro_rules! define_nam_run_traits {
    (NamRun0) => {
        /// Execution trait for 0-argument `#[nam]` functions.
        pub trait NamRun0 {
            /// Executes the nam function across the specified spatial domain in safe mode.
            fn run(self, space: &Space);

            /// Executes the nam function across the specified spatial domain in unchecked mode.
            unsafe fn run_unchecked(self, space: &Space);
        }

        impl<F> NamRun0 for F
        where
            F: Fn(&Space) + 'static,
        {
            #[track_caller]
            #[inline(always)]
            fn run(self, space: &Space) {
                let caller = std::panic::Location::caller();
                dispatch_nam_internal::<F>(self, space, caller, DispatchSafetyMode::Safe, Vec::new(), anu::nam_args_api::IngressContext::new());
            }

            #[track_caller]
            #[inline(always)]
            unsafe fn run_unchecked(self, space: &Space) {
                let caller = std::panic::Location::caller();
                dispatch_nam_internal::<F>(self, space, caller, DispatchSafetyMode::Unchecked, Vec::new(), anu::nam_args_api::IngressContext::new());
            }
        }
    };

    ($(($Trait:ident, $($Arg:ident),*));* $(;)?) => {
        $(
            /// Execution trait for `#[nam]` functions with arguments.
            pub trait $Trait<'a, $($Arg),*> {
                /// Executes the nam function across the specified spatial domain in safe mode.
                ///
                /// # Active Verification
                /// Enforces parameter contracts (`E1001`–`E1004`), domain bounds (`E1008`),
                /// slice range disjointness (`E1007`), temporal presentation mutability (`E1010`),
                /// and forbids unconstrained parallel mutable slices (`E1009`).
                #[allow(non_snake_case)]
                fn run(self, space: &Space, $($Arg: $Arg),*);

                /// Executes the nam function across the specified spatial domain in unchecked mode.
                ///
                /// Bypasses the parallel mutable slice policy (`E1009`), permitting [`SliceMut`]
                /// arguments across multiple threads when writes are coordinated manually.
                ///
                /// # Safety & Active Verification
                /// Does **not** disable all validation: type contracts (`E1001`–`E1004`),
                /// domain bounds (`E1008`), slice range collisions (`E1007`), and temporal presentation
                /// hazards (`E1010`) remain actively enforced by the runtime `BorrowEngine`.
                #[allow(non_snake_case)]
                unsafe fn run_unchecked(self, space: &Space, $($Arg: $Arg),*);
            }

            impl<'a, 'target, F, $($Arg),*> $Trait<'a, $($Arg),*> for F
            where
                $($Arg: GpuTypeMatch<'target>),*,
                F: Fn(&Space, $($Arg::Target),*) + 'static,
            {
                #[track_caller]
                #[inline(always)]
                #[allow(non_snake_case)]
                #[allow(unused_assignments)]
                fn run(self, space: &Space, $($Arg: $Arg),*) {
                    let caller = std::panic::Location::caller();
                    let mut ctx = anu::nam_args_api::IngressContext::new();
                    let mut inputs = Vec::new();
                    let mut arg_idx = 0;
                    $(
                        ctx.current_arg_index = arg_idx;
                        $Arg.collect(&mut ctx);
                        inputs.push($Arg.as_input_record(arg_idx));
                        arg_idx += 1;
                    )*
                    dispatch_nam_internal::<F>(self, space, caller, DispatchSafetyMode::Safe, inputs, ctx);
                }

                #[track_caller]
                #[inline(always)]
                #[allow(non_snake_case)]
                #[allow(unused_assignments)]
                unsafe fn run_unchecked(self, space: &Space, $($Arg: $Arg),*) {
                    let caller = std::panic::Location::caller();
                    let mut ctx = anu::nam_args_api::IngressContext::new();
                    let mut inputs = Vec::new();
                    let mut arg_idx = 0;
                    $(
                        ctx.current_arg_index = arg_idx;
                        $Arg.collect(&mut ctx);
                        inputs.push($Arg.as_input_record(arg_idx));
                        arg_idx += 1;
                    )*
                    dispatch_nam_internal::<F>(self, space, caller, DispatchSafetyMode::Unchecked, inputs, ctx);
                }
            }
        )*
    };
}

#[track_caller]
#[inline(always)]
fn dispatch_nam_internal<F: 'static>(
    _func: F,
    space: &Space,
    caller: &'static std::panic::Location<'static>,
    safety_mode: DispatchSafetyMode,
    inputs: Vec<anu::nam_args_api::InputResourceRecord>,
    ctx: anu::nam_args_api::IngressContext<'static>,
) {
    with_active_flow_at(caller, |flow| {
        if flow.sticky_error.is_some() {
            return;
        }

        let space_contract = SpaceContract::new(space.size_x, space.size_y, space.size_z);
        let call_site = Some((caller.file(), caller.line(), caller.column()));

        let mut map = NamDispatchMap::new(String::new(), safety_mode, space_contract, call_site);
        map.inputs = inputs;

        if let Err(e) = flow.nam_impl_direct::<F>(space, ctx, map) {
            flow.sticky_error = Some(e);
        }
    });
}

define_nam_run_traits!(NamRun0);

define_nam_run_traits! {
    (NamRun1, T0);
    (NamRun2, T0, T1);
    (NamRun3, T0, T1, T2);
    (NamRun4, T0, T1, T2, T3);
    (NamRun5, T0, T1, T2, T3, T4);
    (NamRun6, T0, T1, T2, T3, T4, T5);
    (NamRun7, T0, T1, T2, T3, T4, T5, T6);
    (NamRun8, T0, T1, T2, T3, T4, T5, T6, T7);
    (NamRun9, T0, T1, T2, T3, T4, T5, T6, T7, T8);
    (NamRun10, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9);
    (NamRun11, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
    (NamRun12, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
    (NamRun13, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
    (NamRun14, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
    (NamRun15, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
    (NamRun16, T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
}
