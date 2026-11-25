//! A table's rows read a window at a time, played on a database of its
//! own made with the records' table as the studio makes it. What a window
//! comes to through the daemon, planned for a table it knows, is the
//! studio's and the API suite's (`window.rs`, `bigtable`).

mod read;
