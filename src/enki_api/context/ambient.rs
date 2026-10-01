use std::cell::Cell;
use std::panic::Location;

use super::errors::emit_and_abort;
use super::flow::Flow;

thread_local! {
    static ACTIVE_FLOW_PTR: Cell<*mut Flow<'static>> = const { Cell::new(std::ptr::null_mut()) };

    static ACTIVE_FLOW_CALLER: Cell<Option<&'static Location<'static>>> = const { Cell::new(None) };
}

pub(crate) struct ActiveFlowGuard;

impl ActiveFlowGuard {
    #[track_caller]
    pub fn enter(flow: &mut Flow<'_>) -> Self {
        let caller = Location::caller();

        ACTIVE_FLOW_PTR.with(|cell| {
            if !cell.get().is_null() {
                let diag = anu::diagnostics::rt::nested_frame(caller);
                emit_and_abort(&diag);
            }
            cell.set(flow as *mut Flow<'_> as *mut Flow<'static>);
        });

        ACTIVE_FLOW_CALLER.with(|cell| {
            cell.set(Some(caller));
        });

        Self
    }
}

impl Drop for ActiveFlowGuard {
    fn drop(&mut self) {
        ACTIVE_FLOW_PTR.with(|cell| {
            cell.set(std::ptr::null_mut());
        });
        ACTIVE_FLOW_CALLER.with(|cell| {
            cell.set(None);
        });
    }
}

#[inline(always)]
pub(crate) fn is_inside_active_flow() -> bool {
    ACTIVE_FLOW_PTR.with(|cell| !cell.get().is_null())
}

#[inline(always)]
pub(crate) fn with_active_flow_at<R, F>(caller: &'static Location<'static>, f: F) -> R
where
    F: FnOnce(&mut Flow<'_>) -> R,
{
    ACTIVE_FLOW_PTR.with(|cell| {
        let ptr = cell.get();
        if ptr.is_null() {
            let user_caller = resolve_user_caller(caller);
            let diag = anu::diagnostics::rt::outside_frame(user_caller);
            emit_and_abort(&diag);
        }
        let flow = unsafe { &mut *ptr };
        f(flow)
    })
}

#[track_caller]
#[inline(always)]
pub(crate) fn with_active_flow<R, F>(f: F) -> R
where
    F: FnOnce(&mut Flow<'_>) -> R,
{
    let caller = Location::caller();
    with_active_flow_at(caller, f)
}

#[inline(always)]
pub(crate) fn resolve_user_caller(
    caller: &'static Location<'static>,
) -> &'static Location<'static> {
    let file = caller.file();
    let is_internal = file.contains("enki_api")
        || file.contains("format.rs")
        || file.contains("state.rs")
        || file.contains("core.rs")
        || file.contains("ambient.rs")
        || file.contains("flow.rs")
        || file.contains("nam_run.rs")
        || file.contains("transfer.rs")
        || file.contains("ops.rs")
        || file.contains("lifecycle.rs");

    if is_internal {
        ACTIVE_FLOW_CALLER.with(|c| c.get()).unwrap_or(caller)
    } else {
        caller
    }
}
