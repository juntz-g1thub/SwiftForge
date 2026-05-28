pub mod bash;
pub mod edit;
pub mod grep;
pub mod read;
pub mod write;

pub use bash::BashTool;
pub use edit::EditTool;
pub use grep::GrepTool;
pub use read::ReadTool;
pub use write::WriteTool;

pub fn default_registry() -> swiftforge_types::ToolRegistry {
    use swiftforge_types::ToolRegistry;
    let mut registry = ToolRegistry::new();
    registry.register(BashTool::new());
    registry.register(ReadTool::new());
    registry.register(WriteTool::new());
    registry.register(EditTool::new());
    registry.register(GrepTool::new());
    registry
}