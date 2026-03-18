use crate::{
    app::{LineDrag, RectDrag, Tool, WindowId},
    core::engine::EngineCommand,
};

use super::{
    ingame::{MiddlePanDrag, ScrollbarAxis, ScrollbarDrag},
    AppContext, InGameScreen,
};

impl InGameScreen {
    fn title_close_hit(&self, id: WindowId, col: u16, row: u16) -> bool {
        let win = self.desktop.window(id);
        win.closable
            && row == win.y
            && col >= win.x + win.width.saturating_sub(5)
            && col < win.x + win.width
    }

    fn power_picker_button_at(&self, col: u16, row: u16) -> Option<Tool> {
        let win = self.desktop.window(WindowId::PowerPicker);
        if !win.visible {
            return None;
        }
        let inner_x = win.x.saturating_add(2);
        let inner_y = win.y.saturating_add(2);
        if row == inner_y + 3 && col >= inner_x && col < inner_x + 20 {
            Some(Tool::PowerPlantCoal)
        } else if row == inner_y + 8 && col >= inner_x && col < inner_x + 20 {
            Some(Tool::PowerPlantGas)
        } else {
            None
        }
    }

    fn toolbar_tool_at(&self, col: u16, row: u16) -> Option<Tool> {
        let panel = self.desktop.window(WindowId::Panel);
        if !panel.visible || col < panel.x + 1 || row < panel.y + 1 {
            return None;
        }
        let toolbar_row = row.saturating_sub(panel.y + 1);
        crate::ui::game::toolbar::tool_at_row(toolbar_row)
    }

    pub fn place_current_tool(&mut self, context: &AppContext) {
        let x = self.camera.cursor_x;
        let y = self.camera.cursor_y;
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceTool {
                tool: self.current_tool,
                x,
                y,
            });
        }
        self.message = None;
    }

    pub fn screen_to_map_clamped(&self, col: u16, row: u16, context: &AppContext) -> (usize, usize) {
        let sx = col - self.ui_areas.map.viewport.x;
        let sy = row - self.ui_areas.map.viewport.y;
        let (mx, my) = self.camera.screen_to_map(sx, sy);
        let engine = context.engine.read().unwrap();
        (
            mx.min(engine.map.width.saturating_sub(1)),
            my.min(engine.map.height.saturating_sub(1)),
        )
    }

    pub fn commit_line_drag(&mut self, context: &AppContext) {
        let drag = match self.line_drag.take() {
            Some(drag) => drag,
            None => return,
        };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceLine {
                tool: drag.tool,
                path: drag.path,
            });
        }
        self.message = None;
    }

    pub fn commit_rect_drag(&mut self, context: &AppContext) {
        let drag = match self.rect_drag.take() {
            Some(drag) => drag,
            None => return,
        };
        if let Some(tx) = context.cmd_tx {
            let _ = tx.send(EngineCommand::PlaceRect {
                tool: drag.tool,
                tiles: drag.tiles_cache,
            });
        }
        self.message = None;
    }

    pub fn pan_camera(&mut self, dx: i32, dy: i32, context: &AppContext) {
        let (mw, mh) = {
            let engine = context.engine.read().unwrap();
            (engine.map.width, engine.map.height)
        };
        self.camera.pan(dx, dy, mw, mh);
    }

    pub fn handle_scrollbar_click(&mut self, col: u16, row: u16, context: &AppContext) -> bool {
        let map_ui = self.ui_areas.map;
        let viewport_tiles_w = (map_ui.viewport.width as usize / 2).max(1);
        let viewport_tiles_h = map_ui.viewport.height as usize;

        if map_ui.vertical_bar.contains(col, row) {
            if map_ui.vertical_dec.contains(col, row) {
                self.pan_camera(0, -1, context);
            } else if map_ui.vertical_inc.contains(col, row) {
                self.pan_camera(0, 1, context);
            } else if map_ui.vertical_thumb.contains(col, row) {
                self.scrollbar_drag = Some(ScrollbarDrag {
                    axis: ScrollbarAxis::Vertical,
                    grab_offset: row.saturating_sub(map_ui.vertical_thumb.y),
                });
            } else if map_ui.vertical_track.contains(col, row) {
                let step = viewport_tiles_h.saturating_sub(2).max(1) as i32;
                if row < map_ui.vertical_thumb.y {
                    self.pan_camera(0, -step, context);
                } else if row >= map_ui.vertical_thumb.y + map_ui.vertical_thumb.height {
                    self.pan_camera(0, step, context);
                }
            }
            return true;
        }

        if map_ui.horizontal_bar.contains(col, row) {
            if map_ui.horizontal_dec.contains(col, row) {
                self.pan_camera(-1, 0, context);
            } else if map_ui.horizontal_inc.contains(col, row) {
                self.pan_camera(1, 0, context);
            } else if map_ui.horizontal_thumb.contains(col, row) {
                self.scrollbar_drag = Some(ScrollbarDrag {
                    axis: ScrollbarAxis::Horizontal,
                    grab_offset: col.saturating_sub(map_ui.horizontal_thumb.x),
                });
            } else if map_ui.horizontal_track.contains(col, row) {
                let step = viewport_tiles_w.saturating_sub(2).max(1) as i32;
                if col < map_ui.horizontal_thumb.x {
                    self.pan_camera(-step, 0, context);
                } else if col >= map_ui.horizontal_thumb.x + map_ui.horizontal_thumb.width {
                    self.pan_camera(step, 0, context);
                }
            }
            return true;
        }

        map_ui.corner.contains(col, row)
    }

    pub fn drag_scrollbar_thumb(&mut self, col: u16, row: u16, context: &AppContext) {
        let Some(drag) = self.scrollbar_drag else { return };
        let (map_w, map_h) = {
            let engine = context.engine.read().unwrap();
            (engine.map.width, engine.map.height)
        };
        let viewport_tiles_w = (self.ui_areas.map.viewport.width as usize / 2).max(1);
        let viewport_tiles_h = self.ui_areas.map.viewport.height as usize;
        match drag.axis {
            ScrollbarAxis::Vertical => {
                let track = self.ui_areas.map.vertical_track;
                let thumb = self.ui_areas.map.vertical_thumb;
                if track.height == 0 || thumb.height == 0 || viewport_tiles_h == 0 {
                    return;
                }
                let max_offset = map_h.saturating_sub(viewport_tiles_h);
                let pointer = row.saturating_sub(track.y);
                let offset = crate::ui::game::map_view::scrollbar_offset_from_pointer(
                    track.height,
                    thumb.height,
                    max_offset,
                    pointer,
                    drag.grab_offset,
                );
                self.camera.offset_y = offset as i32;
            }
            ScrollbarAxis::Horizontal => {
                let track = self.ui_areas.map.horizontal_track;
                let thumb = self.ui_areas.map.horizontal_thumb;
                if track.width == 0 || thumb.width == 0 || viewport_tiles_w == 0 {
                    return;
                }
                let max_offset = map_w.saturating_sub(viewport_tiles_w);
                let pointer = col.saturating_sub(track.x);
                let offset = crate::ui::game::map_view::scrollbar_offset_from_pointer(
                    track.width,
                    thumb.width,
                    max_offset,
                    pointer,
                    drag.grab_offset,
                );
                self.camera.offset_x = offset as i32;
            }
        }
    }

    pub fn start_middle_pan(&mut self, col: u16, row: u16) {
        self.map_pan_drag = Some(MiddlePanDrag {
            last_col: col,
            last_row: row,
            carry_cols: 0,
        });
    }

    pub fn drag_middle_pan(&mut self, col: u16, row: u16, context: &AppContext) {
        let Some(mut drag) = self.map_pan_drag else { return };
        let delta_cols = col as i32 - drag.last_col as i32;
        let delta_rows = row as i32 - drag.last_row as i32;
        drag.last_col = col;
        drag.last_row = row;
        drag.carry_cols += delta_cols;

        let tile_dx = drag.carry_cols / 2;
        drag.carry_cols -= tile_dx * 2;
        self.map_pan_drag = Some(drag);

        if tile_dx != 0 || delta_rows != 0 {
            self.pan_camera(-tile_dx, -delta_rows, context);
        }
    }

    pub fn update_line_drag_message(&mut self, context: &AppContext) {
        if let Some(ref drag) = self.line_drag {
            let tool = drag.tool;
            let engine = context.engine.read().unwrap();
            let placeable = drag.path.iter()
                .filter(|&&(x, y)| x < engine.map.width
                    && y < engine.map.height
                    && tool.can_place(engine.map.get(x, y)))
                .count();
            let blocked = drag.path.len() - placeable;
            let cost = placeable as i64 * tool.cost();
            let name = tool.label();
            self.message = Some(if blocked > 0 {
                format!("{name}: {placeable} tiles  ${cost} ({blocked} blocked)")
            } else {
                format!("{name}: {placeable} tiles  ${cost}")
            });
        }
    }

    pub fn update_rect_drag_message(&mut self, context: &AppContext) {
        if let Some(ref drag) = self.rect_drag {
            let tool = drag.tool;
            let engine = context.engine.read().unwrap();
            let placeable = drag.tiles_cache.iter()
                .filter(|&&(x, y)| x < engine.map.width
                    && y < engine.map.height
                    && tool.can_place(engine.map.get(x, y)))
                .count();
            let blocked = drag.tiles_cache.len() - placeable;
            let cost = placeable as i64 * tool.cost();
            let (w, h) = (drag.width(), drag.height());
            self.message = Some(if blocked > 0 {
                format!("{}: {}×{} = {} tiles  ${} ({} blocked)", tool.label(), w, h, placeable, cost, blocked)
            } else {
                format!("{}: {}×{} = {} tiles  ${}", tool.label(), w, h, placeable, cost)
            });
        }
    }

    pub fn handle_click(&mut self, col: u16, row: u16, is_click: bool, context: &AppContext) {
        if self.ui_areas.pause_btn.contains(col, row) {
            if is_click {
                self.paused = !self.paused;
                if let Some(tx) = context.cmd_tx {
                    let _ = tx.send(EngineCommand::SetPaused(self.paused));
                }
            }
            return;
        }

        if self.desktop.contains(WindowId::Panel, col, row)
            || self.desktop.contains(WindowId::Budget, col, row)
            || self.desktop.contains(WindowId::Inspect, col, row)
            || self.desktop.contains(WindowId::PowerPicker, col, row)
        {
            return;
        }

        let engine = context.engine.read().unwrap();
        if self.ui_areas.map.viewport.contains(col, row) {
            let sx = col - self.ui_areas.map.viewport.x;
            let sy = row - self.ui_areas.map.viewport.y;
            let (mx, my) = self.camera.screen_to_map(sx, sy);
            let mx = mx.min(engine.map.width.saturating_sub(1));
            let my = my.min(engine.map.height.saturating_sub(1));
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;

            if self.current_tool == Tool::Inspect {
                self.inspect_pos = Some((mx, my));
                self.desktop.open(WindowId::Inspect, false);
            } else {
                drop(engine);
                self.place_current_tool(context);
            }
            return;
        }

        if self.ui_areas.minimap.contains(col, row) {
            let mm = self.ui_areas.minimap;
            if row == mm.y || mm.height <= 1 {
                return;
            }
            let (mw, mh) = (engine.map.width, engine.map.height);
            let rc = (col - mm.x) as usize;
            let rr = (row - mm.y - 1) as usize;
            let rw = mm.width as usize;
            let rh = (mm.height - 1) as usize;
            let tile_x = if rw <= 1 { 0 } else { rc * (mw - 1) / (rw - 1) };
            let tile_y = if rh <= 1 { 0 } else { rr * (mh - 1) / (rh - 1) };
            let new_ox = tile_x as i32 - self.camera.view_w as i32 / 2;
            let new_oy = tile_y as i32 - self.camera.view_h as i32 / 2;
            self.camera.offset_x = new_ox.clamp(0, (mw as i32 - self.camera.view_w as i32).max(0));
            self.camera.offset_y = new_oy.clamp(0, (mh as i32 - self.camera.view_h as i32).max(0));
        }
    }

    pub fn handle_mouse_click_action(&mut self, col: u16, row: u16, context: &AppContext) -> bool {
        if self.is_budget_open() {
            if self.title_close_hit(WindowId::Budget, col, row) {
                self.close_budget();
                return true;
            }
            if self.desktop.begin_drag(WindowId::Budget, col, row) {
                return true;
            }
            if self.desktop.contains(WindowId::Budget, col, row) {
                self.focus_budget_control_at(col, row);
            }
            return true;
        }
        if self.is_inspect_open() {
            if self.title_close_hit(WindowId::Inspect, col, row) {
                self.inspect_pos = None;
                self.desktop.close(WindowId::Inspect);
                return true;
            }
            if self.desktop.begin_drag(WindowId::Inspect, col, row) {
                return true;
            }
        }
        if self.is_power_picker_open() {
            if self.title_close_hit(WindowId::PowerPicker, col, row) {
                self.desktop.close(WindowId::PowerPicker);
                return true;
            }
            if let Some(tool) = self.power_picker_button_at(col, row) {
                self.select_tool(tool);
                return true;
            }
        }
        if let Some(tool) = self.toolbar_tool_at(col, row) {
            if tool == Tool::PowerPlantPicker {
                self.desktop.toggle(WindowId::PowerPicker, true);
            } else {
                self.select_tool(tool);
            }
            return true;
        }
        if self.desktop.begin_drag(WindowId::Panel, col, row) {
            return true;
        }
        if self.desktop.begin_drag(WindowId::Map, col, row) {
            return true;
        }
        if self.desktop.contains(WindowId::PowerPicker, col, row) {
            return true;
        }
        if self.handle_scrollbar_click(col, row, context) {
            return true;
        }
        if self.is_over_window(col, row) {
            self.handle_click(col, row, true, context);
            return true;
        }
        if Tool::uses_line_drag(self.current_tool) && self.ui_areas.map.viewport.contains(col, row) {
            let (mx, my) = self.screen_to_map_clamped(col, row, context);
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;
            self.line_drag = Some(LineDrag::new(self.current_tool, mx, my));
            self.update_line_drag_message(context);
        } else if Tool::uses_rect_drag(self.current_tool) && self.ui_areas.map.viewport.contains(col, row) {
            let (mx, my) = self.screen_to_map_clamped(col, row, context);
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;
            self.rect_drag = Some(RectDrag::new(self.current_tool, mx, my));
            self.update_rect_drag_message(context);
        } else {
            self.handle_click(col, row, true, context);
        }
        true
    }

    pub fn handle_mouse_drag_action(&mut self, col: u16, row: u16, context: &AppContext) -> bool {
        if self.desktop.update_drag(col, row) {
            return true;
        }
        if self.scrollbar_drag.is_some() {
            self.drag_scrollbar_thumb(col, row, context);
            return true;
        }
        if self.is_over_window(col, row) {
            return true;
        }
        if self.line_drag.is_some() && self.ui_areas.map.viewport.contains(col, row) {
            let (mx, my) = self.screen_to_map_clamped(col, row, context);
            let (tool, sx, sy) = self.line_drag.as_ref().map(|drag| (drag.tool, drag.start_x, drag.start_y)).unwrap();
            let new_path = {
                let engine = context.engine.read().unwrap();
                crate::app::line_drag::line_shortest_path(&engine.map, tool, sx, sy, mx, my)
            };
            if let Some(ref mut drag) = self.line_drag {
                drag.end_x = mx;
                drag.end_y = my;
                drag.path = new_path;
            }
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;
            self.update_line_drag_message(context);
        } else if self.rect_drag.is_some() && self.ui_areas.map.viewport.contains(col, row) {
            let (mx, my) = self.screen_to_map_clamped(col, row, context);
            if let Some(ref mut drag) = self.rect_drag {
                drag.update_end(mx, my);
            }
            self.camera.cursor_x = mx;
            self.camera.cursor_y = my;
            self.update_rect_drag_message(context);
        } else if self.line_drag.is_none() && self.rect_drag.is_none() {
            self.handle_click(col, row, false, context);
        }
        true
    }

    pub fn handle_mouse_up_action(&mut self, col: u16, row: u16, context: &AppContext) -> bool {
        self.desktop.end_drag();
        if self.scrollbar_drag.take().is_some() {
            return true;
        }
        if self.line_drag.is_some() {
            if self.ui_areas.map.viewport.contains(col, row) && !self.is_over_window(col, row) {
                let (mx, my) = self.screen_to_map_clamped(col, row, context);
                let (tool, sx, sy) = self.line_drag.as_ref().map(|drag| (drag.tool, drag.start_x, drag.start_y)).unwrap();
                let final_path = {
                    let engine = context.engine.read().unwrap();
                    crate::app::line_drag::line_shortest_path(&engine.map, tool, sx, sy, mx, my)
                };
                if let Some(ref mut drag) = self.line_drag {
                    drag.end_x = mx;
                    drag.end_y = my;
                    drag.path = final_path;
                }
            }
            self.commit_line_drag(context);
        } else if self.rect_drag.is_some() {
            if self.ui_areas.map.viewport.contains(col, row) && !self.is_over_window(col, row) {
                let (mx, my) = self.screen_to_map_clamped(col, row, context);
                if let Some(ref mut drag) = self.rect_drag {
                    drag.update_end(mx, my);
                }
            }
            self.commit_rect_drag(context);
        }
        true
    }
}
