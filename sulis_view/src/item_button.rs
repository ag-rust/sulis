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

use std::fmt::Display;
use std::any::Any;
use std::rc::Rc;
use std::cell::RefCell;

use sulis_rules::bonus::{AttackBuilder, AttackKindBuilder, Contingent};
use sulis_rules::{Bonus, BonusList, Armor, DamageKind, QuickSlot, Slot};
use sulis_module::{ability, item::{format_item_value, format_item_weight}, Module, PrereqList};
use sulis_state::{EntityState, GameState, ItemState, inventory::has_proficiency};
use sulis_core::io::event;
use sulis_core::ui::{Callback, Widget, WidgetKind, WidgetState};
use sulis_widgets::{Label, TextArea};
use {ItemActionMenu, MerchantWindow, PropWindow, RootView};

enum Kind {
    Prop { index: usize },
    Merchant { id: String },
    Inventory { player: Rc<RefCell<EntityState>> },
    Equipped { player: Rc<RefCell<EntityState>> },
}

pub struct ItemButton {
    icon: String,
    quantity: u32,
    item_index: usize,
    kind: Kind,
    actions: Vec<(String, Callback)>,

    item_window: Option<Rc<RefCell<Widget>>>,
}

const ITEM_BUTTON_NAME: &str = "item_button";

impl ItemButton {
    pub fn inventory(player: &Rc<RefCell<EntityState>>, icon: String,
                     quantity: u32, item_index: usize) -> Rc<RefCell<ItemButton>> {
        let player = Rc::clone(player);
        ItemButton::new(icon, quantity, item_index, Kind::Inventory { player })
    }

    pub fn equipped(player: &Rc<RefCell<EntityState>>, icon: String,
                    item_index: usize) -> Rc<RefCell<ItemButton>> {
        let player = Rc::clone(player);
        ItemButton::new(icon, 1, item_index, Kind::Equipped { player })
    }

    pub fn prop(icon: String, quantity: u32, item_index: usize,
                prop_index: usize) -> Rc<RefCell<ItemButton>> {
        ItemButton::new(icon, quantity, item_index, Kind::Prop { index: prop_index })
    }

    pub fn merchant(icon: String, quantity: u32, item_index: usize,
                    merchant_id: &str) -> Rc<RefCell<ItemButton>> {
        ItemButton::new(icon, quantity, item_index, Kind::Merchant { id: merchant_id.to_string() })
    }

    fn new(icon: String, quantity: u32, item_index: usize, kind: Kind) -> Rc<RefCell<ItemButton>> {
        Rc::new(RefCell::new(ItemButton {
            icon,
            quantity,
            item_index,
            kind,
            actions: Vec::new(),
            item_window: None,
        }))
    }

    pub fn add_action(&mut self, name: &str, cb: Callback) {
        self.actions.push((name.to_string(), cb));
    }

    fn remove_item_window(&mut self) {
        if self.item_window.is_some() {
            self.item_window.as_ref().unwrap().borrow_mut().mark_for_removal();
            self.item_window = None;
        }
    }

    fn get_item_state(&self) -> Option<ItemState> {
        let area_state = GameState::area_state();
        let area_state = area_state.borrow();
        match self.kind {
            Kind::Equipped { ref player } | Kind::Inventory { ref player } => {
                let pc = player.borrow();
                match pc.actor.inventory().items.get(self.item_index) {
                    None => None,
                    Some(&(_, ref item_state)) => Some(item_state.clone())
                }
            }, Kind::Prop { index } => {
                if !area_state.prop_index_valid(index) { return None; }

                match area_state.get_prop(index).items() {
                    None => None,
                    Some(ref items) => {
                        match items.get(self.item_index) {
                            None => None,
                            Some(&(_, ref item_state)) => Some(item_state.clone())
                        }
                    }
                }
            }, Kind::Merchant { ref id } => {
                let merchant = area_state.get_merchant(id);
                let merchant = match merchant {
                    None => return None,
                    Some(ref merchant) => merchant,
                };

                match merchant.items().get(self.item_index) {
                    None => None,
                    Some(&(_, ref item_state)) => Some(item_state.clone())
                }
            }
        }
    }

    fn check_sell_action(&self, widget: &Rc<RefCell<Widget>>) -> Option<Callback> {
        match self.kind {
            Kind::Inventory { .. } => (),
            _ => return None
        }

        // TODO this is a hack putting this here.  but, the state of the merchant
        // window may change after the owing inventory window is opened
        let root = Widget::get_root(widget);
        let root_view = Widget::downcast_kind_mut::<RootView>(&root);
        if let Some(window_widget) = root_view.get_merchant_window(&root) {
            let merchant_window = Widget::downcast_kind_mut::<MerchantWindow>(&window_widget);
            Some(sell_item_cb(merchant_window.player(), self.item_index))
        } else {
            None
        }
    }

    fn add_price_text_arg(&self, root: &Rc<RefCell<Widget>>, item_window: &mut Widget,
                          item_state: &ItemState) {
        let area_state = GameState::area_state();
        let area_state = area_state.borrow();
        match self.kind {
            Kind::Merchant { ref id } => {
                let merchant = area_state.get_merchant(id);
                if let Some(ref merchant) = merchant {
                    let value = merchant.get_buy_price(item_state);
                    item_window.state.add_text_arg("price", &format_item_value(value));
                }
            }, Kind::Inventory { .. } | Kind::Equipped { .. } => {
                let root_view = Widget::downcast_kind_mut::<RootView>(&root);
                let merch_window = match root_view.get_merchant_window(&root) {
                    None => return,
                    Some(window) => window,
                };
                let window = Widget::downcast_kind_mut::<MerchantWindow>(&merch_window);
                let merchant = area_state.get_merchant(window.merchant_id());
                if let Some(ref merchant) = merchant {
                    let value = merchant.get_sell_price(item_state);
                    item_window.state.add_text_arg("price", &format_item_value(value));
                }
            }, _ => (),
        }
    }
}

impl WidgetKind for ItemButton {
    widget_kind!(ITEM_BUTTON_NAME);

    fn on_remove(&mut self) {
        self.remove_item_window();
    }

    fn on_add(&mut self, _widget: &Rc<RefCell<Widget>>) -> Vec<Rc<RefCell<Widget>>> {
        let qty_label = Widget::with_theme(Label::empty(), "quantity_label");
        if self.quantity > 1 {
            qty_label.borrow_mut().state.add_text_arg("quantity", &self.quantity.to_string());
        }
        let icon = Widget::empty("icon");
        icon.borrow_mut().state.add_text_arg("icon", &self.icon);

        vec![icon, qty_label]
    }

    fn on_mouse_enter(&mut self, widget: &Rc<RefCell<Widget>>) -> bool {
        self.super_on_mouse_enter(widget);

        if self.item_window.is_some() { return true; }

        let item_state = self.get_item_state();
        let item_state = match item_state {
            None => return true,
            Some(item_state) => item_state,
        };

        let root = Widget::get_root(widget);
        let item_window = Widget::with_theme(TextArea::empty(), "item_window");
        {
            let mut item_window = item_window.borrow_mut();
            item_window.state.disable();
            item_window.state.set_position(widget.borrow().state.inner_right(),
            widget.borrow().state.inner_top());

            match self.kind {
                Kind::Inventory { ref player } => {
                    if !has_proficiency(&item_state, &player.borrow().actor.stats) {
                        item_window.state.add_text_arg("prof_not_met", "true");
                    }

                    if !item_state.item.meets_prereqs(&player.borrow().actor.actor) {
                        item_window.state.add_text_arg("prereqs_not_met", "true");
                    }
                }, Kind::Merchant { .. } => {
                    let player = GameState::selected();
                    if player.len() > 0 {
                        if !has_proficiency(&item_state, &player[0].borrow().actor.stats) {
                            item_window.state.add_text_arg("prof_not_met", "true");
                        }

                        if !item_state.item.meets_prereqs(&player[0].borrow().actor.actor) {
                            item_window.state.add_text_arg("prereqs_not_met", "true");
                        }
                    }
                },
                _ => (),
            }

            item_window.state.add_text_arg("name", &item_state.item.name);
            item_window.state.add_text_arg("value", &format_item_value(item_state.item.value));
            item_window.state.add_text_arg("weight", &format_item_weight(item_state.item.weight));
            self.add_price_text_arg(&root, &mut item_window, &item_state);

            if let Some(ref prereqs) = &item_state.item.prereqs {
                add_prereq_text_args(prereqs, &mut item_window.state);
            }

            match &item_state.item.usable {
                None => (),
                Some(usable) => {
                    let state = &mut item_window.state;

                    let ap = usable.ap / Module::rules().display_ap;
                    state.add_text_arg("usable_ap", &ap.to_string());
                    if usable.consumable {
                        state.add_text_arg("consumable", "true");
                    }
                    match usable.duration {
                        ability::Duration::Rounds(rounds) =>
                            state.add_text_arg("usable_duration", &rounds.to_string()),
                        ability::Duration::Mode => state.add_text_arg("usable_mode", "true"),
                        ability::Duration::Instant => state.add_text_arg("usable_instant", "true"),
                        ability::Duration::Permanent => state.add_text_arg("usable_permanent", "true"),
                    }
                    state.add_text_arg("usable_description", &usable.short_description);
                }
            }

            match item_state.item.equippable {
                None => (),
                Some(ref equippable) => {
                    if let Some(ref attack) = equippable.attack {
                        add_attack_text_args(attack, &mut item_window.state);
                    }
                    add_bonus_text_args(&equippable.bonuses, &mut item_window.state);
                },
            }
        }
        Widget::add_child_to(&root, Rc::clone(&item_window));
        self.item_window = Some(item_window);

        true
    }

    fn on_mouse_exit(&mut self, widget: &Rc<RefCell<Widget>>) -> bool {
        self.super_on_mouse_exit(widget);

        self.remove_item_window();
        true
    }

    fn on_mouse_release(&mut self, widget: &Rc<RefCell<Widget>>, kind: event::ClickKind) -> bool {
        self.super_on_mouse_release(widget, kind);
        self.remove_item_window();

        match kind {
            event::ClickKind::Left => {
                let action = match self.check_sell_action(widget) {
                    Some(action) => Some(action),
                    None => {
                        match self.actions.first() {
                            None => None,
                            Some(ref action) => Some(action.1.clone()),
                        }
                    }
                };

                if let Some(action) = action {
                    action.call(widget, self);
                }
            },
            event::ClickKind::Right => {
                let menu = ItemActionMenu::new();

                let mut at_least_one_action = false;
                if let Some(action) = self.check_sell_action(widget) {
                    menu.borrow_mut().add_action("Sell", action);
                    at_least_one_action = true;
                }

                for &(ref name, ref cb) in self.actions.iter() {
                    menu.borrow_mut().add_action(name, cb.clone());
                    at_least_one_action = true;
                }

                if at_least_one_action {
                    let menu = Widget::with_defaults(menu);
                    menu.borrow_mut().state.set_modal(true);
                    menu.borrow_mut().state.modal_remove_on_click_outside = true;
                    let root = Widget::get_root(widget);
                    Widget::add_child_to(&root, menu);
                }
            },
            _ => return false,
        }

        true
    }
}

pub fn clear_quickslot_cb(entity: &Rc<RefCell<EntityState>>, slot: QuickSlot) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |_, _| {
        let actor = &mut entity.borrow_mut().actor;
        actor.clear_quick(slot);
    }))
}

pub fn set_quickslot_cb(entity: &Rc<RefCell<EntityState>>,
                        index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |_, _| {
        let actor = &mut entity.borrow_mut().actor;
        for slot in QuickSlot::usable_iter() {
            if actor.inventory().get_quick(*slot).is_none() {
                actor.set_quick(index, *slot);
                return;
            }
        }

        actor.set_quick(index, QuickSlot::Usable1);
    }))
}

pub fn use_item_cb(entity: &Rc<RefCell<EntityState>>, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |widget, _| {
        let root = Widget::get_root(widget);
        let view = Widget::downcast_kind_mut::<RootView>(&root);
        view.set_inventory_window(&root, false);
        GameState::execute_item_on_activate(&entity, index);
    }))
}

pub fn take_item_cb(entity: &Rc<RefCell<EntityState>>,
                    prop_index: usize, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::with(Box::new(move || {
        entity.borrow_mut().actor.take(prop_index, index);
    }))
}

pub fn equip_item_cb(entity: &Rc<RefCell<EntityState>>, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::with(Box::new(move || {
        // equip with no preferred slot
        entity.borrow_mut().actor.equip(index, None);
    }))
}

pub fn buy_item_cb(entity: &Rc<RefCell<EntityState>>,
                   merchant_id: &str, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    let merchant_id = merchant_id.to_string();
    Callback::with(Box::new(move || {
        let area_state = GameState::area_state();
        let mut area_state = area_state.borrow_mut();

        let mut merchant = area_state.get_merchant_mut(&merchant_id);
        let merchant = match merchant {
            None => return,
            Some(ref mut merchant) => merchant,
        };

        let value = match merchant.items().get(index) {
            None => return,
            Some(&(_, ref item_state)) => merchant.get_buy_price(item_state),
        };

        if entity.borrow().actor.inventory().coins() < value {
            return;
        }

        if let Some(item_state) = merchant.remove(index) {
            entity.borrow_mut().actor.add_coins(-value);
            entity.borrow_mut().actor.add_item(item_state);
        }
    }))
}

pub fn sell_item_cb(entity: &Rc<RefCell<EntityState>>, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |widget, _| {
        let root = Widget::get_root(widget);
        let root_view = Widget::downcast_kind_mut::<RootView>(&root);
        let merchant = match root_view.get_merchant_window(&root) {
            None => return,
            Some(ref window) => {
                let merchant_window = Widget::downcast_kind_mut::<MerchantWindow>(&window);
                merchant_window.merchant_id().to_string()
            }
        };

        let area_state = GameState::area_state();
        let mut area_state = area_state.borrow_mut();
        let mut merchant = area_state.get_merchant_mut(&merchant);
        let merchant = match merchant {
            None => return,
            Some(ref mut merchant) => merchant,
        };

        let item_state = entity.borrow_mut().actor.remove_item(index);
        if let Some(item_state) = item_state {
            let value = merchant.get_sell_price(&item_state);
            entity.borrow_mut().actor.add_coins(value);
            merchant.add(item_state);
        }
    }))
}

pub fn drop_item_cb(entity: &Rc<RefCell<EntityState>>, index: usize) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |widget, _| {
        drop_item(widget, &entity, index);
    }))
}

fn drop_item(widget: &Rc<RefCell<Widget>>, entity: &Rc<RefCell<EntityState>>, index: usize) {
    let root = Widget::get_root(widget);
    let root_view = Widget::downcast_kind_mut::<RootView>(&root);
    match root_view.get_prop_window(&root) {
        None => drop_to_ground(entity, index),
        Some(ref window) => {
            let prop_window = Widget::downcast_kind_mut::<PropWindow>(&window);
            let prop_index = prop_window.prop_index();
            drop_to_prop(entity, index, prop_index);
        }
    }
}

fn drop_to_prop(entity: &Rc<RefCell<EntityState>>, index: usize, prop_index: usize) {
    let area_state = GameState::area_state();
    let mut area_state = area_state.borrow_mut();
    if !area_state.prop_index_valid(prop_index) { return; }

    let prop_state = area_state.get_prop_mut(prop_index);
    let item_state = entity.borrow_mut().actor.remove_item(index);

    if let Some(item_state) = item_state {
        prop_state.add_item(item_state);
    }
}

fn drop_to_ground(entity: &Rc<RefCell<EntityState>>, index: usize) {
    let p = entity.borrow().location.to_point();
    let area_state = GameState::area_state();
    let mut area_state = area_state.borrow_mut();

    area_state.check_create_prop_container_at(p.x, p.y);
    if let Some(ref mut prop) = area_state.prop_mut_at(p.x, p.y) {
        let item_state = entity.borrow_mut().actor.remove_item(index);

        if let Some(item_state) = item_state {
            prop.add_item(item_state);
        }
    }
}

pub fn unequip_and_drop_item_cb(entity: &Rc<RefCell<EntityState>>, slot: Slot) -> Callback {
    let entity = Rc::clone(entity);
    Callback::new(Rc::new(move |widget, _| {
        let index = match entity.borrow_mut().actor.inventory().equipped.get(&slot) {
            None => return,
            Some(index) => *index,
        };

        entity.borrow_mut().actor.unequip(slot);
        drop_item(widget, &entity, index);
    }))
}

pub fn unequip_item_cb(entity: &Rc<RefCell<EntityState>>, slot: Slot) -> Callback {
    let entity = Rc::clone(entity);
    Callback::with(Box::new(move || {
        entity.borrow_mut().actor.unequip(slot);
    }))
}

pub fn add_attack_text_args(attack: &AttackBuilder, widget_state: &mut WidgetState) {
    widget_state.add_text_arg("min_damage", &attack.damage.min.to_string());
    widget_state.add_text_arg("max_damage", &attack.damage.max.to_string());
    if attack.damage.ap > 0 {
        widget_state.add_text_arg("armor_penetration", &attack.damage.ap.to_string());
    }
    add_if_present(widget_state, "damage_kind", attack.damage.kind);

    match attack.kind {
        AttackKindBuilder::Melee { reach } =>
            widget_state.add_text_arg("reach", &reach.to_string()),
            AttackKindBuilder::Ranged { range, .. } =>
                widget_state.add_text_arg("range", &range.to_string()),
    }

    let bonuses = &attack.bonuses;
    add_if_nonzero(widget_state, "attack_crit_threshold", bonuses.crit_threshold as f32);
    add_if_nonzero(widget_state, "attack_hit_threshold", bonuses.hit_threshold as f32);
    add_if_nonzero(widget_state, "attack_graze_threshold", bonuses.graze_threshold as f32);
    add_if_nonzero(widget_state, "attack_graze_multiplier", bonuses.graze_multiplier);
    add_if_nonzero(widget_state, "attack_hit_multiplier", bonuses.hit_multiplier);
    add_if_nonzero(widget_state, "attack_crit_multiplier", bonuses.crit_multiplier);
    add_if_nonzero(widget_state, "attack_melee_accuracy", bonuses.melee_accuracy as f32);
    add_if_nonzero(widget_state, "attack_ranged_accuracy", bonuses.ranged_accuracy as f32);
    add_if_nonzero(widget_state, "attack_spell_accuracy", bonuses.spell_accuracy as f32);

    if let Some(damage) = bonuses.damage {
        widget_state.add_text_arg("attack_min_bonus_damage", &damage.min.to_string());
        widget_state.add_text_arg("attack_max_bonus_damage", &damage.max.to_string());
        if let Some(kind) = damage.kind {
            widget_state.add_text_arg("attack_bonus_damage_kind", &kind.to_string());
        }
    }
}

fn add<T: Display>(widget_state: &mut WidgetState, name: &str, value: T) {
    widget_state.add_text_arg(name, &value.to_string());
}

fn add_bonus(bonus: &Bonus, state: &mut WidgetState, has_accuracy: &mut bool,
             group_uses_index: &mut usize, damage_index: &mut usize, armor: &mut Armor) {
    use sulis_rules::BonusKind::*;
    match &bonus.kind {
        Attribute { attribute, amount } => add(state, &attribute.short_name(), amount),
        ActionPoints(amount) => add(state, "bonus_ap", *amount / Module::rules().display_ap as i32),
        Armor(amount) => armor.add_base(*amount),
        ArmorKind { kind, amount } => armor.add_kind(*kind, *amount),
        Damage(damage) => {
            let index = *damage_index;
            if damage.max > 0 {
                add(state, &format!("min_bonus_damage_{}", index), damage.min);
                add(state, &format!("max_bonus_damage_{}", index), damage.max);
            }
            if damage.ap > 0 {
                add(state, &format!("armor_penetration_{}", index), damage.ap);
            }
            if let Some(kind) = damage.kind {
                add(state, &format!("bonus_damage_kind_{}", index), kind);
            }
            *damage_index += 1;
        },
        Reach(amount) => add(state, "bonus_reach", amount),
        Range(amount) => add(state, "bonus_range", amount),
        Initiative(amount) => add(state, "initiative", amount),
        HitPoints(amount) => add(state, "hit_points", amount),
        MeleeAccuracy(amount) => { add(state, "melee_accuracy", amount); *has_accuracy = true; },
        RangedAccuracy(amount) => { add(state, "ranged_accuracy", amount); *has_accuracy = true; },
        SpellAccuracy(amount) => { add(state, "spell_accuracy", amount); *has_accuracy = true; },
        Defense(amount) => add(state, "defense", amount),
        Fortitude(amount) => add(state, "fortitude", amount),
        Reflex(amount) => add(state, "reflex", amount),
        Will(amount) => add(state, "will", amount),
        Concealment(amount) => add(state, "concealment", amount),
        ConcealmentIgnore(amount) => add(state, "concealment_ignore", amount),
        CritThreshold(amount) => add(state, "crit_threshold", amount),
        HitThreshold(amount) => add(state, "hit_threshold", amount),
        GrazeThreshold(amount) => add(state, "graze_threshold", amount),
        CritMultiplier(amount) => add(state, "crit_multiplier", amount),
        HitMultiplier(amount) => add(state, "hit_multiplier", amount),
        GrazeMultiplier(amount) => add(state, "graze_multiplier", amount),
        MovementRate(amount) => add(state, "movement_rate", amount),
        AttackCost(amount) => {
            let cost = amount / Module::rules().display_ap as i32;
            add(state, "attack_cost", cost);
        },
        GroupUsesPerEncounter { group, amount } => {
            let index = *group_uses_index;
            add(state, &format!("ability_group_{}", index), group);
            add(state, &format!("ability_group_{}_uses_per_encounter", index), amount);
            *group_uses_index += 1;
        }
        ArmorProficiency(armor_kind) => {
            add(state, &format!("armor_proficiency_{:?}", armor_kind), "true");
        },
        WeaponProficiency(weapon_kind) => {
            add(state, &format!("weapon_proficiency_{:?}", weapon_kind), "true");
        },
        FlankingAngle(amount) => add(state, "flanking_angle", amount),
        MoveDisabled => add(state, "move_disabled", true),
        AttackDisabled => add(state, "attack_disabled", true),
        Hidden => add(state, "hidden", true),
    }
}

pub fn add_prereq_text_args(prereqs: &PrereqList, state: &mut WidgetState) {
    state.add_text_arg("prereqs", "true");

    if let Some(ref attrs) = prereqs.attributes {
        for &(attr, amount) in attrs.iter() {
            state.add_text_arg(&format!("prereq_{}", attr.short_name()), &amount.to_string());
        }
    }

    for (index, &(ref class_id, level)) in prereqs.levels.iter().enumerate() {
        let class = match Module::class(class_id) {
            None => {
                warn!("Invalid class '{}' in prereq list", class_id);
                continue;
            }, Some(class) => class,
        };
        state.add_text_arg(&format!("prereq_class_{}", index), &class.name);
        state.add_text_arg(&format!("prereq_level_{}", index), &level.to_string());
    }

    if let Some(total_level) = prereqs.total_level {
        state.add_text_arg("prereq_total_level", &total_level.to_string());
    }

    if let Some(ref race) = prereqs.race {
        state.add_text_arg("prereq_race", &race.id);
    }

    for (index, ref ability_id) in prereqs.abilities.iter().enumerate() {
        let ability = match Module::ability(ability_id) {
            None => {
                warn!("No ability '{}' found for prereq list", ability_id);
                continue;
            }, Some(ability) => ability,
        };

        state.add_text_arg(&format!("prereq_ability_{}", index), &ability.name);
    }
}

pub fn add_bonus_text_args(bonuses: &BonusList, widget_state: &mut WidgetState) {
    let mut group_uses_index = 0;
    let mut damage_index = 0;
    let mut armor = Armor::default();
    let mut has_accuracy = false;
    for bonus in bonuses.iter() {
        match bonus.when {
            Contingent::Always => (),
            _ => continue,
        }
        add_bonus(bonus, widget_state, &mut has_accuracy, &mut group_uses_index,
                  &mut damage_index, &mut armor);
    }

    if has_accuracy {
        add(widget_state, "any_accuracy", "true");
    }

    if !armor.is_empty() {
        add(widget_state, "any_armor", "true");
    }
    if armor.base() > 0 {
        add(widget_state, "armor", armor.base());
    }

    for kind in DamageKind::iter() {
        if !armor.differs_from_base(*kind) { continue; }
        add(widget_state, &format!("armor_{}", kind).to_lowercase(), armor.amount(*kind));
    }
}

fn add_if_nonzero(widget_state: &mut WidgetState, text: &str, val: f32) {
    if val != 0.0 {
        widget_state.add_text_arg(text, &val.to_string());
    }
}

fn add_if_present<T: Display>(widget_state: &mut WidgetState, text: &str, val: Option<T>) {
    if let Some(val) = val {
        widget_state.add_text_arg(text, &val.to_string());
    }
}
