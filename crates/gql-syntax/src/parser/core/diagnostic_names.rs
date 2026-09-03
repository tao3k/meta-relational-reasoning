//! Human-readable keyword names used by core parser diagnostics.
#![forbid(unsafe_code)]

use crate::syntax::Keyword;

pub(in crate::parser) fn keyword_name(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Call => "CALL",
        Keyword::Create => "CREATE",
        Keyword::Drop => "DROP",
        Keyword::Insert => "INSERT",
        Keyword::Delete => "DELETE",
        Keyword::Set => "SET",
        Keyword::Remove => "REMOVE",
        Keyword::Detach => "DETACH",
        Keyword::Nodetach => "NODETACH",
        Keyword::Start => "START",
        Keyword::Transaction => "TRANSACTION",
        Keyword::Read => "READ",
        Keyword::Only => "ONLY",
        Keyword::Write => "WRITE",
        Keyword::Commit => "COMMIT",
        Keyword::Rollback => "ROLLBACK",
        Keyword::Case => "CASE",
        Keyword::When => "WHEN",
        Keyword::Then => "THEN",
        Keyword::Else => "ELSE",
        Keyword::End => "END",
        _ => "reserved",
    }
}
