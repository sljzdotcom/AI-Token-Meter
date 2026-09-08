//! Small bounded VT screen for the pinned CLI's cursor/erase redraw protocol.
//! Styling and OSC titles never contribute text. A later erase invalidates old quota.
use super::CollectionError;

pub fn screen(raw: &str) -> Result<String, CollectionError> {
    observe_frames(raw, |_| Ok(()))
}

pub fn observe_frames(
    raw: &str,
    mut observe: impl FnMut(&str) -> Result<(), CollectionError>,
) -> Result<String, CollectionError> {
    if raw.len() > 512 * 1024 {
        return Err(CollectionError::UnrecognizedOutput);
    }
    let mut rows: Vec<Vec<char>> = vec![Vec::new()];
    let (mut row, mut column) = (0usize, 0usize);
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\x1b' => match chars.next() {
                Some('[') => {
                    let mut parameters = String::new();
                    let mut final_byte = None;
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            final_byte = Some(next);
                            break;
                        }
                        parameters.push(next);
                        if parameters.len() > 64 {
                            return Err(CollectionError::UnrecognizedOutput);
                        }
                    }
                    let Some(op) = final_byte else {
                        break;
                    };
                    if parameters.starts_with(['?', '>', '<']) {
                        continue;
                    }
                    let values: Vec<usize> = parameters
                        .split(';')
                        .map(|p| p.parse().unwrap_or(0))
                        .collect();
                    let n = values[0].max(1);
                    match op {
                        'm' | 'n' | 'c' | 'u' => {}
                        'A' => row = row.saturating_sub(n),
                        'B' => row = (row + n).min(255),
                        'C' => column = (column + n).min(511),
                        'D' => column = column.saturating_sub(n),
                        'G' => column = (n - 1).min(511),
                        'H' | 'f' => {
                            row = (n - 1).min(255);
                            column = values
                                .get(1)
                                .copied()
                                .unwrap_or(1)
                                .max(1)
                                .saturating_sub(1)
                                .min(511);
                        }
                        'J' => match values[0] {
                            2 | 3 => rows.iter_mut().for_each(Vec::clear),
                            0 => {
                                if let Some(line) = rows.get_mut(row) {
                                    line.truncate(column);
                                }
                                rows.iter_mut().skip(row + 1).for_each(Vec::clear);
                            }
                            1 => {
                                rows.iter_mut().take(row).for_each(Vec::clear);
                                if let Some(line) = rows.get_mut(row) {
                                    for c in line.iter_mut().take(column + 1) {
                                        *c = ' ';
                                    }
                                }
                            }
                            _ => return Err(CollectionError::UnrecognizedOutput),
                        },
                        'K' => {
                            if let Some(line) = rows.get_mut(row) {
                                match values[0] {
                                    0 => line.truncate(column),
                                    1 => {
                                        for c in line.iter_mut().take(column + 1) {
                                            *c = ' ';
                                        }
                                    }
                                    2 => line.clear(),
                                    _ => return Err(CollectionError::UnrecognizedOutput),
                                }
                            }
                        }
                        _ => return Err(CollectionError::UnrecognizedOutput),
                    }
                }
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some('\\') => {}
                _ => return Err(CollectionError::UnrecognizedOutput),
            },
            '\r' => column = 0,
            '\n' => {
                row += 1;
                column = 0;
                if row >= 256 {
                    rows.remove(0);
                    row = 255;
                }
            }
            '\x08' => column = column.saturating_sub(1),
            '\t' => column = ((column / 8 + 1) * 8).min(511),
            c if c.is_control() => {}
            c => {
                while rows.len() <= row {
                    rows.push(Vec::new());
                }
                if column >= 512 {
                    return Err(CollectionError::UnrecognizedOutput);
                }
                let length = rows[row].len().max(column + 1);
                rows[row].resize(length, ' ');
                rows[row][column] = c;
                column += 1;
                if c == '╯' {
                    observe(&render(&rows))?;
                }
            }
        }
    }
    Ok(render(&rows))
}

fn render(rows: &[Vec<char>]) -> String {
    rows.iter()
        .map(|line| line.iter().collect::<String>().trim_end().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
