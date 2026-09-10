use std::{
    any::Any,
    ffi::{CString, c_int, c_void},
    ops::ControlFlow,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    ptr::{self, NonNull},
};

use javelin_sys as sys;

use crate::{
    Error,
    bytes::{Captures, Match},
};

#[derive(Debug)]
pub(crate) struct Code {
    raw: NonNull<c_void>,
    pub(crate) capture_count: usize,
}

// Upstream permits concurrent matching on a compiled pattern. Each Rust call
// owns its capture storage and callback state; the handle lives until the last Arc.
unsafe impl Send for Code {}
unsafe impl Sync for Code {}

impl Code {
    pub(crate) fn new(pattern: &str, options: c_int) -> Result<Self, Error> {
        let pattern = CString::new(pattern).map_err(|err| Error::NulByte(err.nul_position()))?;
        let mut raw = ptr::null_mut();
        // The C string and output slot remain live throughout compilation.
        let result = unsafe { sys::jp_pattern_compile(&mut raw, pattern.as_ptr(), options) };
        if result != sys::JP_RESULT_OK {
            return Err(Error::Compile(result));
        }
        let raw = NonNull::new(raw).ok_or(Error::Compile(sys::JP_RESULT_INTERNAL_ERROR))?;
        let mut code = Self {
            raw,
            capture_count: 0,
        };
        // Compilation succeeded, so the handle is live and owned by code.
        code.capture_count = unsafe { sys::jp_get_number_of_captures(code.raw.as_ptr()) } as usize;
        assert!(
            (1..=256).contains(&code.capture_count),
            "invalid native capture count"
        );
        Ok(code)
    }

    pub(crate) fn is_match(&self, subject: &[u8], start: usize) -> bool {
        if start > subject.len() {
            return false;
        }
        // All calls borrow a live handle and a buffer covering the stated length.
        unsafe {
            sys::jp_has_partial_match(
                self.raw.as_ptr(),
                subject.as_ptr().cast(),
                subject.len(),
                start,
            )
        }
    }

    pub(crate) fn is_full_match(&self, subject: &[u8]) -> bool {
        unsafe { sys::jp_has_full_match(self.raw.as_ptr(), subject.as_ptr().cast(), subject.len()) }
    }

    pub(crate) fn captures<'s>(&self, subject: &'s [u8], start: usize) -> Option<Captures<'s>> {
        if start > subject.len() {
            return None;
        }
        // The native single-match API leaves nonparticipating groups untouched.
        let mut pointers = vec![ptr::null(); 2 * self.capture_count];
        let matched = unsafe {
            sys::jp_partial_match(
                self.raw.as_ptr(),
                subject.as_ptr().cast(),
                subject.len(),
                pointers.as_mut_ptr(),
                start,
            )
        };
        matched.then(|| captures_from_pointers(subject, &pointers))
    }

    pub(crate) fn scan<'s, F, B>(
        &self,
        subject: &'s [u8],
        start: usize,
        callback: F,
    ) -> ControlFlow<B>
    where
        F: FnMut(Match<'s>) -> ControlFlow<B>,
    {
        let mut state = ScanState::new(subject, callback, self.capture_count);
        // jp_scan calls the trampoline synchronously and retains no pointers.
        unsafe {
            sys::jp_scan(
                self.raw.as_ptr(),
                subject.as_ptr().cast(),
                subject.len(),
                (&mut state as *mut ScanState<'s, F, B>).cast(),
                Some(on_match::<F, B>),
                start,
            );
        }
        state.finish()
    }

    pub(crate) fn scan_captures<'s, F, B>(
        &self,
        subject: &'s [u8],
        start: usize,
        callback: F,
    ) -> ControlFlow<B>
    where
        F: FnMut(Captures<'s>) -> ControlFlow<B>,
    {
        let mut state = ScanState::new(subject, callback, self.capture_count);
        unsafe {
            sys::jp_scan_captures(
                self.raw.as_ptr(),
                subject.as_ptr().cast(),
                subject.len(),
                (&mut state as *mut ScanState<'s, F, B>).cast(),
                Some(on_captures::<F, B>),
                start,
            );
        }
        state.finish()
    }
}

impl Drop for Code {
    fn drop(&mut self) {
        // Code owns this handle, and no matching calls outlive it.
        unsafe { sys::jp_pattern_free(self.raw.as_ptr()) };
    }
}

fn captures_from_pointers<'s>(subject: &'s [u8], pointers: &[*const c_void]) -> Captures<'s> {
    let ranges = pointers
        .chunks_exact(2)
        .map(|pair| {
            if pair[0].is_null() || pair[1].is_null() {
                return None;
            }
            // Integer offsets avoid pointer subtraction on a native sentinel.
            let start = (pair[0] as usize)
                .checked_sub(subject.as_ptr() as usize)
                .expect("invalid capture start");
            let end = (pair[1] as usize)
                .checked_sub(subject.as_ptr() as usize)
                .expect("invalid capture end");
            let matched = Match::new(subject, start, end);
            Some((matched.start(), matched.end()))
        })
        .collect();
    Captures { subject, ranges }
}

struct ScanState<'s, F, B> {
    subject: &'s [u8],
    callback: F,
    capture_count: usize,
    result: ControlFlow<B>,
    panic: Option<Box<dyn Any + Send>>,
}

impl<'s, F, B> ScanState<'s, F, B> {
    fn new(subject: &'s [u8], callback: F, capture_count: usize) -> Self {
        Self {
            subject,
            callback,
            capture_count,
            result: ControlFlow::Continue(()),
            panic: None,
        }
    }

    fn call(&mut self, invoke: impl FnOnce(&'s [u8], &mut F) -> ControlFlow<B>) -> c_int {
        // Rust panics must be resumed after returning through the native frames.
        match catch_unwind(AssertUnwindSafe(|| {
            invoke(self.subject, &mut self.callback)
        })) {
            Ok(ControlFlow::Continue(())) => 0,
            Ok(ControlFlow::Break(value)) => {
                self.result = ControlFlow::Break(value);
                1
            }
            Err(panic) => {
                self.panic = Some(panic);
                1
            }
        }
    }

    fn finish(self) -> ControlFlow<B> {
        if let Some(panic) = self.panic {
            resume_unwind(panic);
        }
        self.result
    }
}

unsafe extern "C" fn on_match<'s, F, B>(from: usize, to: usize, user: *mut c_void) -> c_int
where
    F: FnMut(Match<'s>) -> ControlFlow<B>,
{
    // user is the matching ScanState, exclusively borrowed for this callback.
    let state = unsafe { &mut *user.cast::<ScanState<'s, F, B>>() };
    state.call(|subject, callback| callback(Match::new(subject, from, to)))
}

unsafe extern "C" fn on_captures<'s, F, B>(
    pointers: *const *const c_void,
    count: usize,
    user: *mut c_void,
) -> c_int
where
    F: FnMut(Captures<'s>) -> ControlFlow<B>,
{
    let state = unsafe { &mut *user.cast::<ScanState<'s, F, B>>() };
    let expected = state.capture_count;
    state.call(|subject, callback| {
        assert_eq!(count, expected, "invalid native capture count");
        // The native scanner supplies 2 * count live pointers for this callback.
        let pointers = unsafe { std::slice::from_raw_parts(pointers, 2 * count) };
        callback(captures_from_pointers(subject, pointers))
    })
}
