// ABOUTME: Post-render junction resolver for box-drawing characters.
// ABOUTME: Scans the buffer after all widgets render and fixes intersection characters.

use boba_core::PostRender;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Returns the directional connections of a box-drawing character.
/// (up, down, left, right)
fn connections(ch: &str) -> (bool, bool, bool, bool) {
    match ch {
        // Thin
        "│" => (true, true, false, false),
        "─" => (false, false, true, true),
        "┌" => (false, true, false, true),
        "┐" => (false, true, true, false),
        "└" => (true, false, false, true),
        "┘" => (true, false, true, false),
        "├" => (true, true, false, true),
        "┤" => (true, true, true, false),
        "┬" => (false, true, true, true),
        "┴" => (true, false, true, true),
        "┼" => (true, true, true, true),

        // Thick
        "┃" => (true, true, false, false),
        "━" => (false, false, true, true),
        "┏" => (false, true, false, true),
        "┓" => (false, true, true, false),
        "┗" => (true, false, false, true),
        "┛" => (true, false, true, false),
        "┣" => (true, true, false, true),
        "┫" => (true, true, true, false),
        "┳" => (false, true, true, true),
        "┻" => (true, false, true, true),
        "╋" => (true, true, true, true),

        // Double
        "║" => (true, true, false, false),
        "═" => (false, false, true, true),
        "╔" => (false, true, false, true),
        "╗" => (false, true, true, false),
        "╚" => (true, false, false, true),
        "╝" => (true, false, true, false),
        "╠" => (true, true, false, true),
        "╣" => (true, true, true, false),
        "╦" => (false, true, true, true),
        "╩" => (true, false, true, true),
        "╬" => (true, true, true, true),

        // Rounded
        "╭" => (false, true, false, true),
        "╮" => (false, true, true, false),
        "╰" => (true, false, false, true),
        "╯" => (true, false, true, false),

        // Mixed thin/thick
        "┍" | "┎" => (false, true, false, true),
        "┑" | "┒" => (false, true, true, false),
        "┕" | "┖" => (true, false, false, true),
        "┙" | "┚" => (true, false, true, false),
        "┝" | "┞" | "┟" | "┠" | "┡" | "┢" => (true, true, false, true),
        "┥" | "┦" | "┧" | "┨" | "┩" | "┪" => (true, true, true, false),
        "┭" | "┮" | "┯" | "┰" | "┱" | "┲" => (false, true, true, true),
        "┵" | "┶" | "┷" | "┸" | "┹" | "┺" => (true, false, true, true),
        "┽" | "┾" | "┿" | "╀" | "╁" | "╂" | "╃" | "╄" | "╅" | "╆" | "╇" | "╈" | "╉"
        | "╊" => (true, true, true, true),

        // Mixed thin/double
        "╒" | "╓" => (false, true, false, true),
        "╕" | "╖" => (false, true, true, false),
        "╘" | "╙" => (true, false, false, true),
        "╛" | "╜" => (true, false, true, false),
        "╞" | "╟" => (true, true, false, true),
        "╡" | "╢" => (true, true, true, false),
        "╤" | "╥" => (false, true, true, true),
        "╧" | "╨" => (true, false, true, true),
        "╪" | "╫" => (true, true, true, true),

        // Dashed/dotted (same connections as thin)
        "┆" | "┇" | "┊" | "┋" | "╎" | "╏" => (true, true, false, false),
        "┄" | "┅" | "┈" | "┉" | "╌" | "╍" => (false, false, true, true),

        _ => (false, false, false, false),
    }
}

/// Given a connection mask (up, down, left, right), return the thin box-drawing character.
/// When mixing weights, we default to thin.
fn junction_char(up: bool, down: bool, left: bool, right: bool) -> Option<&'static str> {
    match (up, down, left, right) {
        (true, true, false, false) => Some("│"),
        (false, false, true, true) => Some("─"),
        (false, true, false, true) => Some("┌"),
        (false, true, true, false) => Some("┐"),
        (true, false, false, true) => Some("└"),
        (true, false, true, false) => Some("┘"),
        (true, true, false, true) => Some("├"),
        (true, true, true, false) => Some("┤"),
        (false, true, true, true) => Some("┬"),
        (true, false, true, true) => Some("┴"),
        (true, true, true, true) => Some("┼"),
        _ => None,
    }
}

/// Scan the buffer and fix box-drawing character intersections.
///
/// For each cell containing a border character, checks its 4 neighbors
/// to determine which directions have connecting border segments, then
/// swaps in the correct junction character if needed.
///
/// Only modifies the character — cell style is left unchanged.
pub fn resolve_junctions(buf: &mut Buffer, area: Rect) {
    for y in area.y..area.bottom() {
        for x in area.x..area.right() {
            let sym = buf[(x, y)].symbol().to_string();
            let (up, down, left, right) = connections(&sym);

            // Skip non-border cells
            if !up && !down && !left && !right {
                continue;
            }

            // Check what each neighbor actually connects back toward this cell
            let has_up = y > area.y && {
                let (_, d, _, _) = connections(buf[(x, y - 1)].symbol());
                d // neighbor above connects downward
            };
            let has_down = y + 1 < area.bottom() && {
                let (u, _, _, _) = connections(buf[(x, y + 1)].symbol());
                u // neighbor below connects upward
            };
            let has_left = x > area.x && {
                let (_, _, _, r) = connections(buf[(x - 1, y)].symbol());
                r // neighbor left connects rightward
            };
            let has_right = x + 1 < area.right() && {
                let (_, _, l, _) = connections(buf[(x + 1, y)].symbol());
                l // neighbor right connects leftward
            };

            // If the actual connections differ from the current character, fix it
            if (has_up, has_down, has_left, has_right) != (up, down, left, right) {
                if let Some(new_ch) = junction_char(has_up, has_down, has_left, has_right) {
                    buf[(x, y)].set_symbol(new_ch);
                }
            }
        }
    }
}

/// Post-render hook that resolves box-drawing junctions after all widgets render.
pub struct JunctionResolver;

impl PostRender for JunctionResolver {
    fn after_view(&self, buf: &mut Buffer, area: Rect) {
        resolve_junctions(buf, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    fn buf_from_lines(lines: &[&str]) -> Buffer {
        let height = lines.len() as u16;
        let width = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0) as u16;
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        for (y, line) in lines.iter().enumerate() {
            for (x, ch) in line.chars().enumerate() {
                buf[(x as u16, y as u16)].set_char(ch);
            }
        }
        buf
    }

    fn buf_to_string(buf: &Buffer, area: Rect) -> String {
        let mut result = String::new();
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                result.push_str(buf[(x, y)].symbol());
            }
            if y < area.bottom() - 1 {
                result.push('\n');
            }
        }
        result
    }

    #[test]
    fn test_connections_thin_vertical() {
        let c = connections("│");
        assert_eq!(c, (true, true, false, false));
    }

    #[test]
    fn test_connections_thin_horizontal() {
        let c = connections("─");
        assert_eq!(c, (false, false, true, true));
    }

    #[test]
    fn test_connections_top_left_corner() {
        let c = connections("┌");
        assert_eq!(c, (false, true, false, true));
    }

    #[test]
    fn test_connections_non_border() {
        let c = connections("A");
        assert_eq!(c, (false, false, false, false));
    }

    #[test]
    fn test_connections_thick_vertical() {
        let c = connections("┃");
        assert_eq!(c, (true, true, false, false));
    }

    #[test]
    fn test_connections_double_horizontal() {
        let c = connections("═");
        assert_eq!(c, (false, false, true, true));
    }

    #[test]
    fn test_connections_rounded_corner() {
        let c = connections("╭");
        assert_eq!(c, (false, true, false, true));
    }

    #[test]
    fn test_t_junction_top() {
        // Two side-by-side boxes sharing a border column.
        // The second box's ┌ overwrites the first box's ┐ at the shared edge.
        // ┌──┌──┐  should become  ┌──┬──┐
        // │  │  │                  │  │  │
        // └──└──┘                  └──┴──┘
        let mut buf = buf_from_lines(&["┌──┌──┐", "│  │  │", "└──└──┘"]);
        let area = Rect::new(0, 0, 7, 3);
        resolve_junctions(&mut buf, area);
        let result = buf_to_string(&buf, area);
        assert_eq!(result, "┌──┬──┐\n│  │  │\n└──┴──┘");
    }

    #[test]
    fn test_left_t_junction() {
        let mut buf = buf_from_lines(&["│ ", "│─", "│ "]);
        let area = Rect::new(0, 0, 2, 3);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(0u16, 1u16)].symbol(), "├");
    }

    #[test]
    fn test_right_t_junction() {
        let mut buf = buf_from_lines(&[" │", "─│", " │"]);
        let area = Rect::new(0, 0, 2, 3);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(1u16, 1u16)].symbol(), "┤");
    }

    #[test]
    fn test_cross_junction() {
        let mut buf = buf_from_lines(&[" │ ", "─│─", " │ "]);
        let area = Rect::new(0, 0, 3, 3);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(1u16, 1u16)].symbol(), "┼");
    }

    #[test]
    fn test_no_change_correct_junction() {
        let mut buf = buf_from_lines(&["─┬─", " │ "]);
        let area = Rect::new(0, 0, 3, 2);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(1u16, 0u16)].symbol(), "┬");
    }

    #[test]
    fn test_non_border_cells_untouched() {
        let mut buf = buf_from_lines(&["Hello"]);
        let area = Rect::new(0, 0, 5, 1);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf_to_string(&buf, area), "Hello");
    }

    #[test]
    fn test_jeff_header_sidebar_junction() {
        let mut buf = buf_from_lines(&["────│───", "    │   "]);
        let area = Rect::new(0, 0, 8, 2);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(4u16, 0u16)].symbol(), "┬");
    }

    #[test]
    fn test_jeff_input_box_left_junction() {
        let mut buf = buf_from_lines(&["│  ", "┌──", "│  ", "└──"]);
        let area = Rect::new(0, 0, 3, 4);
        resolve_junctions(&mut buf, area);
        assert_eq!(buf[(0u16, 1u16)].symbol(), "├");
    }
}
