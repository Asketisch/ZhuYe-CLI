//! 终端基础设施族(restructure 批 4):生命周期 RAII、探测、视口扩缩、
//! 键位表与主题调色板。原为 src 根平铺;旧路径经 `lib.rs` 再导出垫片
//! 保持兼容,新代码请写 `crate::term::<name>`。

pub mod keymap;
pub mod terminal;
pub mod terminal_probe;
pub mod theme;
pub mod viewport;
