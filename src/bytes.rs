//! Regular expressions over byte slices. All match offsets are measured in bytes.

use std::{
    ops::{ControlFlow, Range},
    sync::Arc,
};

use javelin_sys as sys;

use crate::{Error, ffi::Code};

/// A match borrowing its input. The end offset is exclusive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Match<'s> {
    subject: &'s [u8],
    start: usize,
    end: usize,
}

impl<'s> Match<'s> {
    pub(crate) fn new(subject: &'s [u8], start: usize, end: usize) -> Self {
        debug_assert!(
            start <= end && end <= subject.len(),
            "invalid native match range"
        );
        Self {
            subject,
            start,
            end,
        }
    }

    /// Returns the starting byte offset.
    pub fn start(&self) -> usize {
        self.start
    }

    /// Returns the exclusive ending byte offset.
    pub fn end(&self) -> usize {
        self.end
    }

    /// Returns the matched byte range in the original input.
    pub fn range(&self) -> Range<usize> {
        self.start..self.end
    }

    /// Returns the matched bytes.
    pub fn as_bytes(&self) -> &'s [u8] {
        &self.subject[self.start..self.end]
    }
}

/// Capture groups for one match. Group zero is the whole match.
///
/// Captures own their ranges and borrow only the input, so they may be retained
/// after a scan callback returns. Groups are addressed by number.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Captures<'s> {
    pub(crate) subject: &'s [u8],
    pub(crate) ranges: Vec<Option<(usize, usize)>>,
}

impl<'s> Captures<'s> {
    /// Returns a participating group, or `None` for an unmatched or absent group.
    pub fn get(&self, index: usize) -> Option<Match<'s>> {
        let (start, end) = self.ranges.get(index).copied().flatten()?;
        Some(Match::new(self.subject, start, end))
    }

    /// Returns the number of capture groups, including group zero.
    pub fn len(&self) -> usize {
        self.ranges.len()
    }

    /// Returns whether there are no groups. Successful matches include group zero.
    pub fn is_empty(&self) -> bool {
        self.ranges.is_empty()
    }

    /// Visits groups in numerical order, including nonparticipating groups.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = Option<Match<'s>>> + '_ {
        (0..self.len()).map(|index| self.get(index))
    }
}

/// Options for compiling a regex. By default, matching operates on bytes with
/// ASCII character classes and case-sensitive matching.
#[derive(Clone, Debug, Default)]
pub struct RegexBuilder {
    options: i32,
}

impl RegexBuilder {
    /// Creates a builder with the default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Compiles a pattern. Literal NUL bytes must be written as `\x00` escapes.
    pub fn build(&self, pattern: &str) -> Result<Regex, Error> {
        Ok(Regex {
            code: Arc::new(Code::new(pattern, self.options)?),
            pattern: Arc::from(pattern),
        })
    }

    fn flag(&mut self, flag: i32, enabled: bool) -> &mut Self {
        if enabled {
            self.options |= flag;
        } else {
            self.options &= !flag;
        }
        self
    }

    /// Enables case-insensitive matching.
    pub fn caseless(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_IGNORE_CASE, yes)
    }

    /// Allows `^` and `$` to match line boundaries inside the input.
    pub fn multi_line(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_MULTILINE, yes)
    }

    /// Allows `.` to match newlines.
    pub fn dotall(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_DOTALL, yes)
    }

    /// Decodes UTF-8 instead of matching individual bytes. Offsets remain bytes.
    pub fn utf(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_UTF8, yes)
    }

    /// Uses Unicode properties for `\d`, `\w`, `\s`, and word boundaries.
    /// This is independent of UTF-8 decoding and case folding.
    pub fn ucp(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_UCP, yes)
    }

    /// Enables Unicode case folding when case-insensitive matching is enabled.
    pub fn unicode_case(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_UNICODE_CASE, yes)
    }

    /// Reverses the default greediness of quantifiers.
    pub fn ungreedy(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_UNGREEDY, yes)
    }

    /// Requires matches to start at the beginning of the input.
    pub fn anchored(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_ANCHORED, yes)
    }

    /// Parses the pattern using JavelinPattern's glob syntax.
    pub fn glob(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_GLOB_SYNTAX, yes)
    }

    /// Prefers the backtracking processor over automatic processor selection.
    pub fn prefer_backtracking(&mut self, yes: bool) -> &mut Self {
        self.flag(sys::JP_OPTION_PREFER_BACK_TRACKING, yes)
    }

    /// Replaces unused captures with non-capturing groups. Captures needed by
    /// backreferences and recursion are retained. This can change capture counts
    /// and numbering; leave it disabled when extracting capture groups.
    pub fn auto_cluster(&mut self, yes: bool) -> &mut Self {
        // Pattern::AUTO_CLUSTER is not named in the native C header.
        self.flag(0x200, yes)
    }
}

/// A compiled regex. Clones share the native pattern and may match concurrently.
#[derive(Clone, Debug)]
pub struct Regex {
    code: Arc<Code>,
    pattern: Arc<str>,
}

impl Regex {
    /// Compiles a pattern using the default options.
    pub fn new(pattern: &str) -> Result<Self, Error> {
        RegexBuilder::new().build(pattern)
    }

    /// Returns the original pattern.
    pub fn as_str(&self) -> &str {
        &self.pattern
    }

    /// Returns the number of capture groups, including group zero.
    pub fn captures_len(&self) -> usize {
        self.code.capture_count
    }

    /// Returns whether the regex matches anywhere in the input.
    pub fn is_match(&self, subject: &[u8]) -> bool {
        self.is_match_at(subject, 0)
    }

    /// Searches from a byte offset while preserving anchor and lookbehind context.
    /// An offset beyond the input returns `false`.
    pub fn is_match_at(&self, subject: &[u8], start: usize) -> bool {
        self.code.is_match(subject, start)
    }

    /// Returns whether the regex matches the entire input.
    pub fn is_full_match(&self, subject: &[u8]) -> bool {
        self.code.is_full_match(subject)
    }

    /// Finds the first match in the input.
    pub fn find<'s>(&self, subject: &'s [u8]) -> Option<Match<'s>> {
        self.find_at(subject, 0)
    }

    /// Finds the first match at or after a byte offset, preserving input context.
    /// An offset beyond the input returns `None`.
    pub fn find_at<'s>(&self, subject: &'s [u8], start: usize) -> Option<Match<'s>> {
        match self.scan_at(subject, start, ControlFlow::Break) {
            ControlFlow::Break(matched) => Some(matched),
            ControlFlow::Continue(()) => None,
        }
    }

    /// Returns the capture groups of the first match.
    pub fn captures<'s>(&self, subject: &'s [u8]) -> Option<Captures<'s>> {
        self.captures_at(subject, 0)
    }

    /// Returns captures for the first match at or after a byte offset.
    /// Input context is preserved; offsets beyond the input return `None`.
    pub fn captures_at<'s>(&self, subject: &'s [u8], start: usize) -> Option<Captures<'s>> {
        self.code.captures(subject, start)
    }

    /// Visits non-overlapping matches in leftmost-first order.
    ///
    /// Return `ControlFlow::Continue(())` to continue or `ControlFlow::Break(value)`
    /// to stop and return that value. Empty matches are included: after one,
    /// nonempty alternatives at the same position are tried before advancing.
    /// Callbacks may perform nested scans. Callback panics propagate after the
    /// native scan has stopped, without unwinding through C++.
    ///
    /// ```
    /// use std::ops::ControlFlow;
    /// use javelin_pattern::bytes::Regex;
    /// let re = Regex::new(r"\d+")?;
    /// let mut ranges = Vec::new();
    /// let result = re.scan(b"12 and 34", |m| {
    ///     ranges.push(m.range());
    ///     ControlFlow::<()>::Continue(())
    /// });
    /// assert_eq!(result, ControlFlow::Continue(()));
    /// assert_eq!(ranges, [0..2, 7..9]);
    /// # Ok::<(), javelin_pattern::Error>(())
    /// ```
    pub fn scan<'s, F, B>(&self, subject: &'s [u8], callback: F) -> ControlFlow<B>
    where
        F: FnMut(Match<'s>) -> ControlFlow<B>,
    {
        self.scan_at(subject, 0, callback)
    }

    /// Like [`Regex::scan`], starting at a byte offset into the original input.
    /// An offset beyond the input completes without invoking the callback.
    pub fn scan_at<'s, F, B>(&self, subject: &'s [u8], start: usize, callback: F) -> ControlFlow<B>
    where
        F: FnMut(Match<'s>) -> ControlFlow<B>,
    {
        self.code.scan(subject, start, callback)
    }

    /// Visits capture groups with the iteration and stopping rules of [`Regex::scan`].
    /// Capture ranges are copied into Rust-owned storage for each match.
    pub fn scan_captures<'s, F, B>(&self, subject: &'s [u8], callback: F) -> ControlFlow<B>
    where
        F: FnMut(Captures<'s>) -> ControlFlow<B>,
    {
        self.scan_captures_at(subject, 0, callback)
    }

    /// Like [`Regex::scan_captures`], starting at a byte offset into the original input.
    /// An offset beyond the input completes without invoking the callback.
    pub fn scan_captures_at<'s, F, B>(
        &self,
        subject: &'s [u8],
        start: usize,
        callback: F,
    ) -> ControlFlow<B>
    where
        F: FnMut(Captures<'s>) -> ControlFlow<B>,
    {
        self.code.scan_captures(subject, start, callback)
    }
}
