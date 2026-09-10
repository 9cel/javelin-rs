use std::{ffi::c_void, ptr};

use javelin_sys::*;

#[test]
fn bytecode_and_pattern_ownership() {
    unsafe {
        let mut bytecode = ptr::null_mut();
        assert_eq!(
            jp_bytecode_compile(&mut bytecode, c"(ab)".as_ptr(), 0),
            JP_RESULT_OK
        );
        let mut pattern = ptr::null_mut();
        assert_eq!(
            jp_pattern_create(
                &mut pattern,
                jp_bytecode_get_data(bytecode),
                jp_bytecode_get_length(bytecode),
                true
            ),
            JP_RESULT_OK
        );
        jp_bytecode_free(bytecode);
        assert_eq!(jp_get_number_of_captures(pattern), 2);
        let subject = b"ab";
        let mut captures = [ptr::null(); 4];
        assert!(jp_full_match(
            pattern,
            subject.as_ptr().cast(),
            subject.len(),
            captures.as_mut_ptr()
        ));
        assert_eq!(captures[0], subject.as_ptr().cast());
        assert_eq!(captures[1], subject.as_ptr().add(2).cast());
        assert_eq!(captures[2], captures[0]);
        assert_eq!(captures[3], captures[1]);
        jp_pattern_free(pattern);
    }
}

#[test]
fn scan_returns_exact_callback_status() {
    unsafe extern "C" fn on_match(from: usize, to: usize, user: *mut c_void) -> i32 {
        let range = unsafe { &mut *user.cast::<(usize, usize)>() };
        *range = (from, to);
        -27
    }
    unsafe {
        let mut pattern = ptr::null_mut();
        assert_eq!(
            jp_pattern_compile(&mut pattern, c"a".as_ptr(), 0),
            JP_RESULT_OK
        );
        let subject = b"ba";
        let mut range = (usize::MAX, usize::MAX);
        assert_eq!(
            jp_scan(
                pattern,
                subject.as_ptr().cast(),
                subject.len(),
                (&mut range as *mut (usize, usize)).cast(),
                Some(on_match),
                0
            ),
            -27
        );
        assert_eq!(range, (1, 2));
        jp_pattern_free(pattern);
    }
}
