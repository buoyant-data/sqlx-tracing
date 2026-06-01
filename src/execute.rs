//! Helper for forwarding a query to the inner executor after capturing its SQL.
//!
//! `sqlx` 0.9 changed [`sqlx::Execute::sql`] to consume the query by value, so the
//! original value can no longer be both inspected (for the tracing span) and
//! forwarded to the wrapped executor. [`Traced`] re-packages the parts we can
//! recover from any `Execute` value — the SQL text, bound arguments (including any
//! encoding error), and the `persistent` flag — into an owned, forwardable
//! [`sqlx::Execute`].

use sqlx::{Database, Execute, SqlStr};

type BoxDynError = Box<dyn std::error::Error + Send + Sync + 'static>;

/// An owned [`sqlx::Execute`] reconstructed from another query so that the SQL
/// text can be recorded in a tracing span before forwarding to the inner executor.
///
/// Note: any cached prepared statement carried by the original query is dropped;
/// the inner executor re-resolves it from the SQL text.
pub(crate) struct Traced<DB: Database> {
    sql: SqlStr,
    arguments: Option<Result<DB::Arguments, BoxDynError>>,
    persistent: bool,
}

impl<DB: Database> Traced<DB> {
    /// Decompose a query into a forwardable [`Traced`], preserving its arguments
    /// and `persistent` flag.
    pub(crate) fn split<'q, E>(mut query: E) -> Self
    where
        E: Execute<'q, DB>,
    {
        let persistent = query.persistent();
        let arguments = query.take_arguments().transpose();
        let sql = query.sql();
        Self {
            sql,
            arguments,
            persistent,
        }
    }

    /// Borrow the SQL text for use in a tracing span.
    pub(crate) fn sql_str(&self) -> &str {
        self.sql.as_str()
    }
}

impl<'q, DB: Database> Execute<'q, DB> for Traced<DB> {
    fn sql(self) -> SqlStr {
        self.sql
    }

    fn statement(&self) -> Option<&DB::Statement> {
        None
    }

    fn take_arguments(&mut self) -> Result<Option<DB::Arguments>, BoxDynError> {
        self.arguments.take().transpose()
    }

    fn persistent(&self) -> bool {
        self.persistent
    }
}
