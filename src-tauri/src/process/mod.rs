//! 程序資訊：`ps` 解析、前景程式與類型判斷。

pub mod ps;

pub use ps::{classify, foreground_by_tty, parse_ps, snapshot, Foreground, ProcInfo};
