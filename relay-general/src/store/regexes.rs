use once_cell::sync::Lazy;
use regex::Regex;

/// Contains multiple capture groups which will be used as a replace placeholder.
///
/// This regex is inspired by one used for grouping:
/// <https://github.com/getsentry/sentry/blob/6ba59023a78bfe033e48ea4e035b64710a905c6b/src/sentry/grouping/strategies/message.py#L16-L97>
pub static TRANSACTION_NAME_NORMALIZER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?x)
    (?P<uuid>[^/\\]*
        \b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b
    [^/\\]*) |
    (?P<sha1>[^/\\]*
        \b[0-9a-fA-F]{40}\b
    [^/\\]*) |
    (?P<md5>[^/\\]*
        \b[0-9a-fA-F]{32}\b
    [^/\\]*) |
    (?P<date>[^/\\]*
        (?:
            (?:\d{4}-[01]\d-[0-3]\dT[0-2]\d:[0-5]\d:[0-5]\d\.\d+([+-][0-2]\d:[0-5]\d|Z))|
            (?:\d{4}-[01]\d-[0-3]\dT[0-2]\d:[0-5]\d:[0-5]\d([+-][0-2]\d:[0-5]\d|Z))|
            (?:\d{4}-[01]\d-[0-3]\dT[0-2]\d:[0-5]\d([+-][0-2]\d:[0-5]\d|Z))
        ) |
        (?:
            \b(?:(Sun|Mon|Tue|Wed|Thu|Fri|Sat)\s+)?
            (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+
            (?:[\d]{1,2})\s+
            (?:[\d]{2}:[\d]{2}:[\d]{2})\s+
            [\d]{4}
        ) |
        (?:
            \b(?:(Sun|Mon|Tue|Wed|Thu|Fri|Sat),\s+)?
            (?:0[1-9]|[1-2]?[\d]|3[01])\s+
            (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+
            (?:19[\d]{2}|[2-9][\d]{3})\s+
            (?:2[0-3]|[0-1][\d]):([0-5][\d])
            (?::(60|[0-5][\d]))?\s+
            (?:[-\+][\d]{2}[0-5][\d]|(?:UT|GMT|(?:E|C|M|P)(?:ST|DT)|[A-IK-Z]))
        )
    [^/\\]*) |
    (?P<hex>[^/\\]*
        \b0[xX][0-9a-fA-F]+\b
    [^/\\]*) |
    (?:^|[/\\])
    (?P<int>
        (:?[^%/\\]|%[0-9a-fA-F]{2})*\d{2,}
    [^/\\]*)"#,
    )
    .unwrap()
});

/// Regex with multiple capture groups for SQL tokens we should scrub.
///
/// Slightly modified from
/// <https://github.com/getsentry/sentry/blob/244b33e44bbbfa0dd680f5a15053e2efaaf6fd65/src/sentry/spans/grouping/strategy/base.py#L132>
/// <https://github.com/getsentry/sentry/blob/65fb6fdaa0080b824ab71559ce025a9ec6818b3e/src/sentry/spans/grouping/strategy/base.py#L170>
/// <https://github.com/getsentry/sentry/blob/17af7efe869007f85c5322e48aa9f80a8515bde4/src/sentry/spans/grouping/strategy/base.py#L163>
pub static SQL_NORMALIZER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?xi)
    # Capture parameters in `IN` statements.
    ((?-x)IN \((?P<in>(%s|\$?\d+|\?)(\s*,\s*(%s|\$?\d+|\?))*)\)) |
    # Capture `SAVEPOINT` savepoints.
    ((?-x)SAVEPOINT (?P<savepoint>(?:(?:"[^"]+")|(?:'[^']+')|(?:`[^`]+`)|(?:[a-z]\w+)))) |
    # Capture single-quoted strings, including the remaining substring if `\'` is found.
    ((?-x)(?P<single_quoted_strs>('(?:[^']|'')*?(?:\\'.*|[^']')))) |
    # Don't capture double-quoted strings (eg used for identifiers in PostgreSQL).
    # Capture numbers.
    ((?-x)(?P<number>(-?\b(?:[0-9]+\.)?[0-9]+(?:[eE][+-]?[0-9]+)?\b))) |
    # Capture booleans (as full tokens, not as substrings of other tokens).
    ((?-x)(?P<bool>(\b(?:true|false)\b)))
    "#,
    )
    .unwrap()
});

/// Regex with multiple capture groups for cache tokens we should scrub.
///
/// The regex attempts to identify all tokens based on hex chars and segments,
/// excluding the first token. A segment is a string inside curly braces after a
/// separator, for example `notsegment:{segment}:notsegment`.
pub static CACHE_NORMALIZER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?xi)
        # Don't scrub the first segment.
        # Capture hex.
        (([\s.+:/\-])+(?P<hex>[a-fA-F0-9]+\b)+) |
        # Capture segments, in form of`:{hi}:`
        (([\s.+:/\-])+(?P<segment>\{[^\}]*\})+)
    "#,
    )
    .unwrap()
});

/// Regex with multiple capture groups for resource tokens we should scrub.
///
/// Resource tokens are the tokens that exist in resource spans that generate
/// high cardinality or are noise for the product. For example, the hash of the
/// file next to its name.
///
/// Slightly modified Regex from
/// <https://github.com/getsentry/sentry/blob/de5949a9a313d7ef0bf0685f84fe6e981ac38558/src/sentry/utils/performance_issues/base.py#L292-L306>
pub static RESOURCE_NORMALIZER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?xi)
        # UUIDs.
        (?P<uuid>[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}) |
        # Chunks and chunk numbers.
        (?P<chunk>(?:[0-9]+\.)?[a-f0-9]{8}\.chunk) |
        # Trailing hashes before final extension.
        ([-.](?P<trailing_hash>(?:[a-f0-9]{8,64}\.?)+)\.([a-z0-9]{2,6})$) |
        # Versions in the path or filename.
        (?P<version>(v[0-9]+(?:\.[0-9]+)*)) |
        # Larger hex-like hashes (avoid false negatives from above).
        (?P<large_hash>[a-f0-9]{16,64}) |
        # Only numbers (for file names that are just numbers).
        (?P<only_numbers>/[0-9]+(\.[a-z0-9]{2,6})$)
        "#,
    )
    .unwrap()
});
