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
use std::rc::Rc;
use std::cell::RefCell;

use sulis_core::io::event;
use sulis_module::{Conversation, conversation::{MerchantData, OnSelect, Response}, Module};
use sulis_state::{EntityState, ChangeListener, GameState};
use sulis_core::ui::{Widget, WidgetKind};
use sulis_widgets::{Label, TextArea};

use {RootView};

pub const NAME: &str = "dialog_window";

pub struct DialogWindow {
    pc: Rc<RefCell<EntityState>>,
    entity: Rc<RefCell<EntityState>>,
    convo: Rc<Conversation>,
    cur_node: String,

    node: Rc<RefCell<TextArea>>,
}

impl DialogWindow {
    pub fn new(pc: &Rc<RefCell<EntityState>>, entity: &Rc<RefCell<EntityState>>,
               convo: Rc<Conversation>) -> Rc<RefCell<DialogWindow>> {
        let cur_node = convo.initial_node();

        Rc::new(RefCell::new(DialogWindow {
            pc: Rc::clone(pc),
            entity: Rc::clone(entity),
            convo: convo,
            node: TextArea::empty(),
            cur_node,
        }))
    }
}

impl WidgetKind for DialogWindow {
    widget_kind!(NAME);

    fn on_remove(&mut self) {
        self.entity.borrow_mut().actor.listeners.remove(NAME);
    }

    fn on_add(&mut self, widget: &Rc<RefCell<Widget>>) -> Vec<Rc<RefCell<Widget>>> {
        self.entity.borrow_mut().actor.listeners.add(
            ChangeListener::invalidate(NAME, widget));

        let title = Widget::with_theme(Label::empty(), "title");
        title.borrow_mut().state.add_text_arg("name", &self.entity.borrow().actor.actor.name);

        self.node.borrow_mut().text = Some(self.convo.text(&self.cur_node).to_string());
        let node_widget = Widget::with_theme(self.node.clone(), "node");
        for flag in self.entity.borrow().custom_flags() {
            node_widget.borrow_mut().state.add_text_arg(flag, "true");
        }

        if let &Some(ref on_select) = self.convo.on_view(&self.cur_node) {
            activate(widget, on_select, &self.pc, &self.entity);
        }

        let responses = Widget::empty("responses");
        {
            for response in self.convo.responses(&self.cur_node) {
                if !is_viewable(response, &self.pc, &self.entity) { continue; }

                let response_button = ResponseButton::new(&response);
                let widget = Widget::with_defaults(response_button);
                Widget::add_child_to(&responses, widget);
            }
        }

        vec![title, node_widget, responses]
    }
}

struct ResponseButton {
    text: String,
    to: Option<String>,
    on_select: Option<OnSelect>,
}

impl ResponseButton {
    fn new(response: &Response) -> Rc<RefCell<ResponseButton>> {
        Rc::new(RefCell::new(ResponseButton {
            text: response.text.to_string(),
            to: response.to.clone(),
            on_select: response.on_select.clone(),
        }))
    }
}

impl WidgetKind for ResponseButton {
    widget_kind!("response_button");

    fn on_add(&mut self, _widget: &Rc<RefCell<Widget>>) -> Vec<Rc<RefCell<Widget>>> {
        let text_area = Widget::with_defaults(TextArea::new(&self.text));

        vec![text_area]
    }

    fn layout(&mut self, widget: &mut Widget) {
        widget.do_self_layout();

        widget.do_children_layout();
    }

    fn on_mouse_release(&mut self, widget: &Rc<RefCell<Widget>>, kind: event::ClickKind) -> bool {
        self.super_on_mouse_release(widget, kind);

        let parent = Widget::go_up_tree(widget, 2);
        let window = Widget::downcast_kind_mut::<DialogWindow>(&parent);

        if let Some(ref on_select) = self.on_select {
            activate(widget, on_select, &window.pc, &window.entity);
        }

        match self.to {
            None => {
                parent.borrow_mut().mark_for_removal();
            }, Some(ref to) => {
                window.cur_node = to.to_string();
                parent.borrow_mut().invalidate_children();
            }
        }

        true
    }
}

pub fn is_viewable(response: &Response, pc: &Rc<RefCell<EntityState>>,
                   target: &Rc<RefCell<EntityState>>) -> bool {
    if let Some(ref on_select) = response.to_view {
        if let Some(ref flags) = on_select.target_flags {
            for flag in flags.iter() {
                if !target.borrow_mut().has_custom_flag(flag) { return false; }
            }
        }

        if let Some(ref flags) = on_select.player_flags {
            for flag in flags.iter() {
                if !pc.borrow_mut().has_custom_flag(flag) { return false; }
            }
        }
    }

    true
}

pub fn activate(widget: &Rc<RefCell<Widget>>, on_select: &OnSelect,
                pc: &Rc<RefCell<EntityState>>, target: &Rc<RefCell<EntityState>>) {
    if let Some(ref flags) = on_select.target_flags {
        for flag in flags.iter() {
            target.borrow_mut().set_custom_flag(flag);
        }
    }

    if let Some(ref flags) = on_select.player_flags {
        for flag in flags.iter() {
            pc.borrow_mut().set_custom_flag(flag);
        }
    }

    if let Some(ref merch) = on_select.show_merchant {
        show_merchant(widget, merch);
    }
}

fn show_merchant(widget: &Rc<RefCell<Widget>>, merch: &MerchantData) {
    let id = &merch.id;
    let loot = match Module::loot_list(&merch.loot_list) {
        None => {
            warn!("Unable to find loot list '{}' for merchant '{}'", merch.loot_list, id);
            return;
        }, Some(loot) => loot,
    };

    {
        let area_state = GameState::area_state();
        let mut area_state = area_state.borrow_mut();

        area_state.get_or_create_merchant(id, &loot);
    }

    let root = Widget::get_root(widget);
    let root_view = Widget::downcast_kind_mut::<RootView>(&root);
    root_view.set_merchant_window(&root, true, &id);
}