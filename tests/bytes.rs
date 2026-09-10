use std::{
    ops::ControlFlow,
    panic::{AssertUnwindSafe, catch_unwind},
};

use javelin_pattern::{
    Error,
    bytes::{Regex, RegexBuilder},
};

#[test]
fn matches_and_captures_borrow_the_input() {
    let input = b"before answer=42 after";
    let (matched, captures) = {
        let re = Regex::new(r"(\w+)=(\d+)").unwrap();
        assert_eq!(re.as_str(), r"(\w+)=(\d+)");
        assert_eq!(re.captures_len(), 3);
        assert!(re.is_match(input));
        assert!(!re.is_full_match(input));
        assert!(re.is_full_match(b"answer=42"));
        (re.find(input).unwrap(), re.captures(input).unwrap())
    };
    assert_eq!(matched.range(), 7..16);
    assert_eq!(matched.as_bytes(), b"answer=42");
    assert_eq!(captures.get(1).unwrap().as_bytes(), b"answer");
    assert_eq!(captures.get(2).unwrap().as_bytes(), b"42");
    assert_eq!(captures.iter().count(), 3);
    assert!(!captures.is_empty());
    assert!(captures.get(3).is_none());
}

#[test]
fn binary_input_and_compile_errors() {
    let re = Regex::new(r"\x00(\xFF)").unwrap();
    let input = b"x\0\xffy";
    assert_eq!(re.find(input).unwrap().range(), 1..3);
    assert_eq!(
        re.captures(input).unwrap().get(1).unwrap().as_bytes(),
        b"\xff"
    );
    assert_eq!(Regex::new("a\0b").unwrap_err(), Error::NulByte(1));
    let error = Regex::new("(").unwrap_err();
    assert!(error.code().unwrap() > 0);
    assert!(error.to_string().contains("compilation failed"));
}

#[test]
fn offsets_preserve_context_and_handle_end_of_input() {
    let re = Regex::new(r"(?<=a)b").unwrap();
    assert!(re.is_match_at(b"ab", 1));
    assert_eq!(re.find_at(b"ab", 1).unwrap().range(), 1..2);
    assert_eq!(
        re.captures_at(b"ab", 1).unwrap().get(0).unwrap().range(),
        1..2
    );
    assert!(!Regex::new(r"\Ab").unwrap().is_match_at(b"ab", 1));
    for offset in [3, usize::MAX] {
        assert!(!re.is_match_at(b"ab", offset));
        assert!(re.find_at(b"ab", offset).is_none());
        assert!(re.captures_at(b"ab", offset).is_none());
        assert_eq!(
            re.scan_at(b"ab", offset, |_| -> ControlFlow<()> {
                panic!("unexpected match")
            }),
            ControlFlow::Continue(())
        );
        assert_eq!(
            re.scan_captures_at(b"ab", offset, |_| -> ControlFlow<()> {
                panic!("unexpected match")
            }),
            ControlFlow::Continue(())
        );
    }
    let empty = Regex::new("").unwrap();
    assert_eq!(empty.find(b"").unwrap().range(), 0..0);
    assert!(empty.is_match(b""));
    assert!(empty.is_full_match(b""));
    assert_eq!(empty.captures(b"").unwrap().get(0).unwrap().as_bytes(), b"");
    assert_eq!(empty.find_at(b"ab", 2).unwrap().range(), 2..2);
}

#[test]
fn empty_matches_retry_nonempty_alternatives() {
    let re = Regex::new("a*?").unwrap();
    let mut ranges = Vec::new();
    assert_eq!(
        re.scan(b"a", |matched| {
            ranges.push(matched.range());
            ControlFlow::<()>::Continue(())
        }),
        ControlFlow::Continue(())
    );
    assert_eq!(ranges, [0..0, 0..1, 1..1]);
    let mut captures = Vec::new();
    assert_eq!(
        re.scan_captures(b"a", |capture| {
            captures.push(capture);
            ControlFlow::<()>::Continue(())
        }),
        ControlFlow::Continue(())
    );
    assert_eq!(
        captures
            .iter()
            .map(|c| c.get(0).unwrap().range())
            .collect::<Vec<_>>(),
        ranges
    );
}

#[test]
fn scans_clear_unmatched_groups_and_retain_empty_groups() {
    let re = Regex::new("(a)|(b)()").unwrap();
    let mut captures = Vec::new();
    assert_eq!(
        re.scan_captures(b"ab", |capture| {
            captures.push(capture);
            ControlFlow::<()>::Continue(())
        }),
        ControlFlow::Continue(())
    );
    assert_eq!(captures[0].get(1).unwrap().range(), 0..1);
    assert!(captures[0].get(2).is_none());
    assert!(captures[0].get(3).is_none());
    assert!(captures[1].get(1).is_none());
    assert_eq!(captures[1].get(2).unwrap().range(), 1..2);
    assert_eq!(captures[1].get(3).unwrap().range(), 2..2);
}

#[test]
fn early_stopping_returns_payload_and_allows_nested_scans() {
    let re = Regex::new("a").unwrap();
    let mut calls = 0;
    let result = re.scan(b"aaa", |matched| {
        calls += 1;
        assert_eq!(re.find(b"ba").unwrap().range(), 1..2);
        if calls == 2 {
            ControlFlow::Break(matched.range())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert_eq!(result, ControlFlow::Break(1..2));
    assert_eq!(calls, 2);
    assert_eq!(
        re.scan(b"a", |_| ControlFlow::Break(String::from("final"))),
        ControlFlow::Break(String::from("final"))
    );
    assert_eq!(
        re.scan_captures(b"aaa", |c| ControlFlow::Break(c.get(0).unwrap().range())),
        ControlFlow::Break(0..1)
    );
}

#[test]
fn callback_panics_resume_in_rust_and_leave_regex_usable() {
    let re = Regex::new("a").unwrap();
    let panic = catch_unwind(AssertUnwindSafe(|| {
        re.scan(b"aa", |_| -> ControlFlow<()> {
            std::panic::panic_any(123usize)
        })
    }))
    .unwrap_err();
    assert_eq!(panic.downcast_ref::<usize>(), Some(&123));
    let panic = catch_unwind(AssertUnwindSafe(|| {
        re.scan_captures(b"aa", |_| -> ControlFlow<()> {
            std::panic::panic_any(String::from("capture panic"))
        })
    }))
    .unwrap_err();
    assert_eq!(panic.downcast_ref::<String>().unwrap(), "capture panic");
    assert!(re.is_full_match(b"a"));
}

#[test]
fn builder_options_and_unicode_byte_offsets() {
    let re = RegexBuilder::new()
        .caseless(true)
        .multi_line(true)
        .build("^abc$")
        .unwrap();
    assert_eq!(re.find(b"x\nABC\ny").unwrap().range(), 2..5);
    assert!(
        RegexBuilder::new()
            .dotall(true)
            .build("a.b")
            .unwrap()
            .is_full_match(b"a\nb")
    );
    assert!(
        !RegexBuilder::new()
            .caseless(true)
            .caseless(false)
            .build("a")
            .unwrap()
            .is_match(b"A")
    );
    assert_eq!(
        RegexBuilder::new()
            .ungreedy(true)
            .build("a+")
            .unwrap()
            .find(b"aaa")
            .unwrap()
            .range(),
        0..1
    );
    assert!(
        !RegexBuilder::new()
            .anchored(true)
            .build("a")
            .unwrap()
            .is_match(b"ba")
    );
    assert!(
        RegexBuilder::new()
            .glob(true)
            .build("*.txt")
            .unwrap()
            .is_full_match(b"file.txt")
    );

    let re = RegexBuilder::new()
        .utf(true)
        .ucp(true)
        .build(r"\w+")
        .unwrap();
    let input = "!αβ!".as_bytes();
    assert_eq!(re.find(input).unwrap().range(), 1..5);
    assert_eq!(
        re.captures(input).unwrap().get(0).unwrap().as_bytes(),
        "αβ".as_bytes()
    );
    let re = RegexBuilder::new()
        .utf(true)
        .caseless(true)
        .unicode_case(true)
        .build("Σ")
        .unwrap();
    assert!(re.is_full_match("σ".as_bytes()));
    let re = RegexBuilder::new().utf(true).build("").unwrap();
    let mut ranges = Vec::new();
    assert_eq!(
        re.scan("é".as_bytes(), |matched| {
            ranges.push(matched.range());
            ControlFlow::<()>::Continue(())
        }),
        ControlFlow::Continue(())
    );
    assert_eq!(ranges, [0..0, 2..2]);
}

#[test]
fn captures_report_reset_match_start() {
    let re = Regex::new(r"a\Kb").unwrap();
    assert_eq!(re.find(b"ab").unwrap().range(), 1..2);
    assert_eq!(re.captures(b"ab").unwrap().get(0).unwrap().range(), 1..2);
}

#[test]
fn clones_share_a_regex_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<Regex>();
    let re = Regex::new(r"(a+)|(b+)").unwrap();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let re = re.clone();
            std::thread::spawn(move || {
                for _ in 0..32 {
                    assert_eq!(re.find(b"xxaaabb").unwrap().range(), 2..5);
                    let captures = re.captures(b"bb").unwrap();
                    assert!(captures.get(1).is_none());
                    assert_eq!(captures.get(2).unwrap().as_bytes(), b"bb");
                }
            })
        })
        .collect();
    drop(re);
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn unused_captures_can_be_removed_without_losing_backreferences() {
    for backtracking in [false, true] {
        let mut builder = RegexBuilder::new();
        builder.prefer_backtracking(backtracking).auto_cluster(true);
        let re = builder.build("(a)(b)").unwrap();
        assert_eq!(re.captures_len(), 1);
        assert!(re.is_full_match(b"ab"));

        let re = builder.build(r"(a)(b)\2").unwrap();
        assert!(re.is_full_match(b"abb"));
        assert!(!re.is_full_match(b"aba"));

        let re = builder.build("(a*?)").unwrap();
        let mut ranges = Vec::new();
        assert_eq!(
            re.scan(b"a", |matched| {
                ranges.push(matched.range());
                ControlFlow::<()>::Continue(())
            }),
            ControlFlow::Continue(())
        );
        assert_eq!(ranges, [0..0, 0..1, 1..1]);

        let re = builder
            .auto_cluster(false)
            .prefer_backtracking(false)
            .build("(a)(b)")
            .unwrap();
        assert_eq!(re.captures_len(), 3);
        assert_eq!(re.captures(b"ab").unwrap().get(2).unwrap().as_bytes(), b"b");
    }
}
