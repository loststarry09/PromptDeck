/// Pure index arithmetic for keyboard list navigation (`↑`/`↓`).
///
/// `current` is the index of the selected row, or `None` when the selection is
/// absent (empty selection, or the selected Prompt is filtered out of the
/// visible list). `delta` is negative for `↑` and positive for `↓`. Results are
/// clamped to `0..len`; an empty list has no selection.
pub fn stepped_index(current: Option<usize>, len: usize, delta: i32) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let last = len - 1;
    Some(match current {
        None if delta < 0 => last,
        None => 0,
        Some(index) => {
            let index = index.min(last);
            if delta < 0 {
                index.saturating_sub(1)
            } else {
                (index + 1).min(last)
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list_has_no_selection() {
        assert_eq!(stepped_index(None, 0, 1), None);
        assert_eq!(stepped_index(None, 0, -1), None);
        assert_eq!(stepped_index(Some(0), 0, 1), None);
    }

    #[test]
    fn first_step_from_no_selection_enters_the_list() {
        assert_eq!(stepped_index(None, 3, 1), Some(0));
        assert_eq!(stepped_index(None, 3, -1), Some(2));
    }

    #[test]
    fn steps_then_clamps_at_both_ends() {
        assert_eq!(stepped_index(Some(0), 3, 1), Some(1));
        assert_eq!(stepped_index(Some(1), 3, 1), Some(2));
        assert_eq!(stepped_index(Some(2), 3, 1), Some(2));
        assert_eq!(stepped_index(Some(2), 3, -1), Some(1));
        assert_eq!(stepped_index(Some(1), 3, -1), Some(0));
        assert_eq!(stepped_index(Some(0), 3, -1), Some(0));
    }

    #[test]
    fn single_item_stays_selected() {
        assert_eq!(stepped_index(Some(0), 1, 1), Some(0));
        assert_eq!(stepped_index(Some(0), 1, -1), Some(0));
        assert_eq!(stepped_index(None, 1, 1), Some(0));
    }

    #[test]
    fn stale_index_beyond_the_list_is_clamped() {
        assert_eq!(stepped_index(Some(9), 3, 1), Some(2));
        assert_eq!(stepped_index(Some(9), 3, -1), Some(1));
    }
}
