//  This file is part of Sulis, a turn based RPG written in Rust.
//  Copyright 2018 Jared Stephen
//
//  Sulis is free software: you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation, either version 3 of the License, or
//  (at your option) any later version.
//
//  Sulis is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with Sulis.  If not, see <http://www.gnu.org/licenses/>

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use sulis_core::ui::{Callback, Widget, WidgetKind};
use sulis_core::widgets::{Button, TextArea};
use sulis_module::on_trigger::ScriptMenuChoice;
use sulis_state::script::{CallbackData, ScriptCallback, ScriptMenuSelection};

const NAME: &str = "script_menu";

pub struct ScriptMenu {
    callback: CallbackData,
    title: String,
    choices: Vec<ScriptMenuChoice>,
}

impl ScriptMenu {
    pub fn new(
        callback: CallbackData,
        title: String,
        choices: Vec<ScriptMenuChoice>,
    ) -> Rc<RefCell<ScriptMenu>> {
        Rc::new(RefCell::new(ScriptMenu {
            callback,
            title,
            choices,
        }))
    }
}

impl WidgetKind for ScriptMenu {
    widget_kind!(NAME);

    fn on_add(&mut self, widget: &Rc<RefCell<Widget>>) -> Vec<Rc<RefCell<Widget>>> {
        widget.borrow_mut().state.set_modal(true);

        let cancel = Widget::with_theme(Button::empty(), "cancel");
        cancel
            .borrow_mut()
            .state
            .add_callback(Callback::new(Rc::new(|widget, _| {
                let (parent, _) = Widget::parent::<ScriptMenu>(widget);
                parent.borrow_mut().mark_for_removal();
            })));

        let entries = Widget::empty("entries");
        // let scrollpane = ScrollPane::new();
        // let entries = Widget::with_theme(scrollpane.clone(), "entries");
        for choice in self.choices.iter() {
            let text_area = Widget::with_defaults(TextArea::empty());
            text_area
                .borrow_mut()
                .state
                .add_text_arg("choice", &choice.display);

            let widget = Widget::with_theme(Button::empty(), "entry");

            let text = choice.clone();
            let cb = self.callback.clone();
            widget
                .borrow_mut()
                .state
                .add_callback(Callback::new(Rc::new(move |widget, _| {
                    let selection = ScriptMenuSelection {
                        value: text.value.to_string(),
                    };
                    cb.on_menu_select(selection);

                    let (parent, _) = Widget::parent::<ScriptMenu>(widget);
                    parent.borrow_mut().mark_for_removal();
                })));
            Widget::add_child_to(&widget, text_area);
            Widget::add_child_to(&entries, widget);
            // scrollpane.borrow().add_to_content(widget);
        }

        let title = Widget::with_theme(TextArea::empty(), "title");
        title.borrow_mut().state.add_text_arg("title", &self.title);

        vec![title, entries, cancel]
    }
}
