#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    #[expect(dead_code)]
    Tick, // drives the 60fps timer countdown once Running; no-op in Phase 2
    Char(char),
    Backspace,
    Space,
    Tab,
    Esc,
}
