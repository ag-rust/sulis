use std::rc::Rc;
use std::cell::RefCell;
use std::cmp;

use state::{AreaState, GameState};
use ui::{WidgetKind, Widget};
use io::{InputAction, TextRenderer};
use io::event::ClickKind;
use resource::Point;

pub struct AreaWidget<'a> {
    area_state: Rc<RefCell<AreaState<'a>>>,
    mouse_over: Rc<RefCell<Widget<'a>>>,
}

impl<'a> AreaWidget<'a> {
    pub fn new(area_state: &Rc<RefCell<AreaState<'a>>>,
               mouse_over: Rc<RefCell<Widget<'a>>>) -> Rc<AreaWidget<'a>> {
        Rc::new(AreaWidget {
            area_state: Rc::clone(area_state),
            mouse_over: mouse_over,
        })
    }

}

impl<'a> WidgetKind<'a> for AreaWidget<'a> {
    fn get_name(&self) -> &str {
        "Area"
    }

    fn on_add(&self, widget: &mut Widget) {
        let width = self.area_state.borrow().area.width;
        let height = self.area_state.borrow().area.height;
        widget.state.set_max_scroll_pos(width, height);
    }

    fn draw_text_mode(&self, renderer: &mut TextRenderer, widget: &Widget<'a>) {
        let p = widget.state.inner_position;
        let s = widget.state.inner_size;

        let state = self.area_state.borrow();
        let ref area = state.area;

        let max_x = cmp::min(s.width, area.width - widget.state.scroll_pos.x);
        let max_y = cmp::min(s.height, area.height - widget.state.scroll_pos.y);

        renderer.set_cursor_pos(0, 0);

        for y in 0..max_y {
            renderer.set_cursor_pos(p.x, p.y + y);
            for x in 0..max_x {
                renderer.render_char(state.get_display(x + widget.state.scroll_pos.x,
                                                       y + widget.state.scroll_pos.y));
            }
        }
    }

    fn on_key_press(&self, _state: &mut GameState, widget: &mut Widget<'a>,
                    key: InputAction, _mouse_pos: Point) -> bool {

        use io::InputAction::*;
        match key {
           ScrollUp => widget.state.scroll(0, -1),
           ScrollDown => widget.state.scroll(0, 1),
           ScrollLeft => widget.state.scroll(-1, 0),
           ScrollRight => widget.state.scroll(1, 0),
           _ => false,
        };
        true
    }

    fn on_mouse_click(&self, state: &mut GameState, widget: &mut Widget<'a>,
                _kind: ClickKind, mouse_pos: Point) -> bool {
        let size = state.pc().size();
        let pos = &widget.state.position;
        let x = (mouse_pos.x - pos.x) - size / 2;
        let y = (mouse_pos.y - pos.y) - size / 2;
        if x >= 0 && y >= 0 {
            state.pc_move_to(x + widget.state.scroll_pos.x, y +
                             widget.state.scroll_pos.y);
        }

        true
    }

    fn on_mouse_move(&self, _state: &mut GameState, widget: &mut Widget<'a>,
                      mouse_pos: Point) -> bool {
        self.super_on_mouse_enter(widget);
        self.mouse_over.borrow_mut().state.set_text(&format!("[{},{}]",
            mouse_pos.x, mouse_pos.y));
        true
    }

    fn on_mouse_exit(&self, _state: &mut GameState, widget: &mut Widget<'a>,
                     _mouse_pos: Point) -> bool {
        self.super_on_mouse_exit(widget);
        self.mouse_over.borrow_mut().state.set_text("");
        true
    }
}
