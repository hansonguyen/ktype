use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Tick(Duration),
    Char(char),
    Backspace,
    Space,
    Tab,
    ShiftTab,
    Left,
    Right,
    Esc,
    EndTest,
    UpdateAvailable(String),
    TogglePunctuation,
    ToggleNumbers,
}
