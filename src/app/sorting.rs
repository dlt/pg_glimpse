//! Sorting and filtering logic for table views.

use crate::db::models::{
    ActiveQuery, IndexInfo, PgExtension, PgSetting, StatStatement, TableSchema, TableStat,
};

/// Trait for types that can be filtered with fuzzy matching.
pub trait Filterable {
    fn filter_string(&self) -> String;
}

impl Filterable for ActiveQuery {
    fn filter_string(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.pid,
            self.usename.as_deref().unwrap_or(""),
            self.datname.as_deref().unwrap_or(""),
            self.state.as_deref().unwrap_or(""),
            self.wait_event.as_deref().unwrap_or(""),
            self.query.as_deref().unwrap_or(""),
        )
    }
}

impl Filterable for IndexInfo {
    fn filter_string(&self) -> String {
        format!(
            "{} {} {} {}",
            self.schemaname, self.table_name, self.index_name, self.index_definition
        )
    }
}

impl Filterable for StatStatement {
    fn filter_string(&self) -> String {
        self.query.clone()
    }
}

impl Filterable for TableStat {
    fn filter_string(&self) -> String {
        format!("{} {}", self.schemaname, self.relname)
    }
}

impl Filterable for PgSetting {
    fn filter_string(&self) -> String {
        format!("{} {} {}", self.name, self.category, self.short_desc)
    }
}

impl Filterable for PgExtension {
    fn filter_string(&self) -> String {
        format!(
            "{} {} {}",
            self.name,
            self.schema,
            self.description.as_deref().unwrap_or("")
        )
    }
}

impl Filterable for TableSchema {
    fn filter_string(&self) -> String {
        format!("{} {}", self.schema_name, self.table_name)
    }
}

/// Trait for sort column enums to enable generic `TableViewState`
pub trait SortColumnTrait: Copy + PartialEq {
    fn next(self) -> Self;
    #[allow(dead_code)]
    fn label(self) -> &'static str;
}

/// Macro to define sort column enums with cycling and labels.
/// Generates: enum definition, `next()` cycling through variants in order, `label()` for display.
macro_rules! define_sort_column {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub fn next(self) -> Self {
                let idx = Self::ALL.iter().position(|&v| v == self).unwrap_or(0);
                Self::ALL[(idx + 1) % Self::ALL.len()]
            }

            #[allow(dead_code)]
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label),+
                }
            }
        }

        impl SortColumnTrait for $name {
            fn next(self) -> Self { Self::next(self) }
            fn label(self) -> &'static str { Self::label(self) }
        }
    };
}

define_sort_column!(SortColumn {
    Duration => "Duration",
    Pid => "PID",
    User => "User",
    State => "State",
});

define_sort_column!(IndexSortColumn {
    Scans => "Scans",
    Size => "Size",
    Name => "Name",
    TupRead => "Tup Read",
    TupFetch => "Tup Fetch",
});

define_sort_column!(TableStatSortColumn {
    DeadTuples => "Dead Tuples",
    Size => "Size",
    Name => "Name",
    SeqScan => "Seq Scan",
    IdxScan => "Idx Scan",
    DeadRatio => "Dead %",
});

define_sort_column!(StatementSortColumn {
    TotalTime => "Total Time",
    MeanTime => "Mean Time",
    MaxTime => "Max Time",
    Stddev => "Stddev",
    Calls => "Calls",
    Rows => "Rows",
    HitRatio => "Hit %",
    SharedReads => "Reads",
    IoTime => "I/O Time",
    Temp => "Temp",
});

/// Sort indices by a key extracted from items. Handles ascending/descending.
pub fn sort_by_key<T, K: Ord>(indices: &mut [usize], items: &[T], asc: bool, key: impl Fn(&T) -> K) {
    indices.sort_by(|&a, &b| {
        let cmp = key(&items[a]).cmp(&key(&items[b]));
        if asc {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

/// Sort indices by a key extracted from items, using `partial_cmp` for floats.
pub fn sort_by_key_partial<T, K: PartialOrd>(
    indices: &mut [usize],
    items: &[T],
    asc: bool,
    key: impl Fn(&T) -> K,
) {
    indices.sort_by(|&a, &b| {
        let cmp = key(&items[a])
            .partial_cmp(&key(&items[b]))
            .unwrap_or(std::cmp::Ordering::Equal);
        if asc {
            cmp
        } else {
            cmp.reverse()
        }
    });
}
