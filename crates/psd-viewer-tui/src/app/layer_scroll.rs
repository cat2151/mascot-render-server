use ratatui::widgets::ListState;

use super::App;

impl App {
    pub(crate) fn layer_list_state(&mut self, viewport_height: u16) -> ListState {
        let total_items = self.layer_list_item_count();
        if total_items == 0 {
            self.layer_scroll_offset = 0;
            return ListState::default();
        }

        let visible_rows = usize::from(viewport_height.max(1));
        let max_offset = total_items.saturating_sub(visible_rows);
        self.layer_scroll_offset = self.layer_scroll_offset.min(max_offset);

        let mut state = ListState::default().with_offset(self.layer_scroll_offset);
        let Some(selected_item_index) = self.selected_layer_item_index() else {
            self.layer_scroll_offset = 0;
            return state.with_offset(0);
        };

        let margin_rows = layer_scroll_margin_rows(visible_rows, self.layer_scroll_margin_ratio);
        self.layer_scroll_offset = adjust_scroll_offset(
            self.layer_scroll_offset,
            selected_item_index,
            total_items,
            visible_rows,
            margin_rows,
        );

        state.select(Some(selected_item_index));
        *state.offset_mut() = self.layer_scroll_offset;
        state
    }

    fn selected_layer_item_index(&self) -> Option<usize> {
        self.selected_layer_selection()
            .map(|index| index + self.layer_list_header_count())
    }

    fn layer_list_item_count(&self) -> usize {
        if self.layer_rows.is_empty() {
            1
        } else {
            self.layer_rows.len() + self.layer_list_header_count()
        }
    }

    fn layer_list_header_count(&self) -> usize {
        if self.layer_rows.is_empty() || self.selected_psd_entry().is_none() {
            0
        } else {
            2
        }
    }
}

fn layer_scroll_margin_rows(visible_rows: usize, ratio: f32) -> usize {
    let max_margin = visible_rows.saturating_sub(1) / 2;
    (((visible_rows as f32) * ratio).floor() as usize).min(max_margin)
}

fn adjust_scroll_offset(
    current_offset: usize,
    selected_index: usize,
    total_items: usize,
    visible_rows: usize,
    margin_rows: usize,
) -> usize {
    if visible_rows == 0 || total_items <= visible_rows {
        return 0;
    }

    let max_offset = total_items.saturating_sub(visible_rows);
    let lower_bound = current_offset.saturating_add(margin_rows);
    let upper_bound = current_offset
        .saturating_add(visible_rows.saturating_sub(1))
        .saturating_sub(margin_rows);

    if selected_index < lower_bound {
        selected_index.saturating_sub(margin_rows).min(max_offset)
    } else if selected_index > upper_bound {
        selected_index
            .saturating_add(margin_rows)
            .saturating_add(1)
            .saturating_sub(visible_rows)
            .min(max_offset)
    } else {
        current_offset.min(max_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::{adjust_scroll_offset, layer_scroll_margin_rows};

    #[test]
    fn margin_rows_use_ratio_and_keep_center_capacity() {
        assert_eq!(layer_scroll_margin_rows(8, 0.25), 2);
        assert_eq!(layer_scroll_margin_rows(9, 0.33), 2);
        assert_eq!(layer_scroll_margin_rows(3, 0.49), 1);
        assert_eq!(layer_scroll_margin_rows(2, 0.49), 0);
    }

    #[test]
    fn moving_up_from_bottom_keeps_bottom_context_visible() {
        let offset = adjust_scroll_offset(22, 27, 30, 8, 2);
        assert_eq!(offset, 22);
    }
}
