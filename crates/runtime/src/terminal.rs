//! 把终端控制序列转换为适合 GUI 展示的纯文本。

pub(super) fn sanitize_terminal_text(text: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Text,
        Escape,
        EscapeIntermediate,
        ControlSequence,
        ControlString,
        ControlStringEscape,
    }

    let mut sanitized = String::with_capacity(text.len());
    let mut state = State::Text;

    for character in text.chars() {
        state = match state {
            State::Text => match character {
                '\u{1b}' => State::Escape,
                '\t' => {
                    sanitized.push(character);
                    State::Text
                }
                character if character.is_control() => State::Text,
                _ => {
                    sanitized.push(character);
                    State::Text
                }
            },
            State::Escape => match character {
                '[' => State::ControlSequence,
                ']' | 'P' | '^' | '_' => State::ControlString,
                '\u{20}'..='\u{2f}' => State::EscapeIntermediate,
                _ => State::Text,
            },
            State::EscapeIntermediate => {
                if ('\u{30}'..='\u{7e}').contains(&character) {
                    State::Text
                } else {
                    State::EscapeIntermediate
                }
            }
            State::ControlSequence => {
                if ('\u{40}'..='\u{7e}').contains(&character) {
                    State::Text
                } else if character == '\u{1b}' {
                    State::Escape
                } else {
                    State::ControlSequence
                }
            }
            State::ControlString => match character {
                '\u{07}' => State::Text,
                '\u{1b}' => State::ControlStringEscape,
                _ => State::ControlString,
            },
            State::ControlStringEscape => {
                if character == '\\' {
                    State::Text
                } else {
                    State::ControlString
                }
            }
        };
    }

    sanitized
}
