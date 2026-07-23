//! 集中式 HTTP/API 契约测试入口。
//!
//! 新增端点测试应作为 `api` 下按 `method_path` 命名的子模块加入，避免每个端点生成一个
//! 顶层 Cargo integration-test target。

mod support;

#[path = "api/mod.rs"]
mod api;
