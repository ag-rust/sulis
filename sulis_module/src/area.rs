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

mod layer;
pub use self::layer::Layer;

mod layer_set;
pub use self::layer_set::LayerSet;

mod path_finder_grid;
use self::path_finder_grid::PathFinderGrid;

pub mod tile;
pub use self::tile::Tile;
pub use self::tile::Tileset;

use std::collections::{HashSet, HashMap};
use std::io::{Error, ErrorKind};
use std::rc::Rc;

use serde::{Deserialize, Deserializer, Serializer};
use serde::ser::{SerializeMap, SerializeStruct};

use sulis_core::image::Image;
use sulis_core::resource::{ResourceSet, Sprite};
use sulis_core::util::{Point, Size, unable_to_create_error};

use crate::{Encounter, Module, ObjectSize, OnTrigger, Prop, ItemListEntrySaveState};

pub const MAX_AREA_SIZE: i32 = 128;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum TriggerKind {
    OnCampaignStart,
    OnAreaLoad,
    OnPlayerEnter { location: Point, size: Size },
    OnEncounterCleared { encounter_location: Point },
    OnEncounterActivated { encounter_location: Point },
}

#[derive(Debug, Clone)]
pub struct Trigger {
    pub kind: TriggerKind,
    pub on_activate: Vec<OnTrigger>,
    pub initially_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub from: Point,
    pub size: Rc<ObjectSize>,
    pub to: ToKind,
    pub hover_text: String,
    pub image_display: Rc<Image>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct ActorData {
    pub id: String,
    pub location: Point,
    pub unique_id: Option<String>,
}

pub struct PropData {
    pub prop: Rc<Prop>,
    pub location: Point,
    pub items: Vec<ItemListEntrySaveState>,
    pub enabled: bool,
    pub hover_text: Option<String>,
}

pub struct EncounterData {
    pub encounter: Rc<Encounter>,
    pub location: Point,
    pub size: Size,
    pub triggers: Vec<usize>,
}

pub struct Area {
    pub id: String,
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub layer_set: LayerSet,
    path_grids: HashMap<String, PathFinderGrid>,
    pub visibility_tile: Rc<Sprite>,
    pub explored_tile: Rc<Sprite>,
    pub actors: Vec<ActorData>,
    pub props: Vec<PropData>,
    pub transitions: Vec<Transition>,
    pub encounters: Vec<EncounterData>,
    pub triggers: Vec<Trigger>,
    pub vis_dist: i32,
    pub vis_dist_squared: i32,
    pub vis_dist_up_one_squared: i32,
    pub world_map_location: Option<String>,
    pub on_rest: OnRest,
    pub location_kind: LocationKind,
}

impl PartialEq for Area {
    fn eq(&self, other: &Area) -> bool {
        self.id == other.id
    }
}

impl Area {
    pub fn new(builder: AreaBuilder, module: &Module) -> Result<Area, Error> {
        let mut props = Vec::new();
        for prop_builder in builder.props.iter() {
            let prop_data = create_prop(prop_builder, module)?;
            props.push(prop_data);
        }

        info!("Creating area {}", builder.id);
        let layer_set = LayerSet::new(&builder, module, &props);
        let layer_set = match layer_set {
            Ok(l) => l,
            Err(e) => {
                warn!("Unable to generate layer_set for area '{}'", builder.id);
                return Err(e);
            }
        };

        let mut path_grids = HashMap::new();
        for (_, size) in module.sizes.iter() {
            let path_grid = PathFinderGrid::new(Rc::clone(size), &layer_set);
            debug!("Generated path grid for size {}", size.id);
            path_grids.insert(size.id.to_string(), path_grid);
        }

        // TODO validate position of all actors, props, encounters

        let mut transitions: Vec<Transition> = Vec::new();
        for (index, t_builder) in builder.transitions.into_iter().enumerate() {
            let image = match ResourceSet::get_image(&t_builder.image_display) {
                None => {
                    warn!("Image '{}' not found for transition.", t_builder.image_display);
                    continue;
                },
                Some(image) => image,
            };

            let size = match module.sizes.get(&t_builder.size) {
                None => {
                    warn!("Size '{}' not found for transition.", t_builder.size);
                    continue;
                }, Some(ref size) => Rc::clone(size),
            };

            let p = t_builder.from;
            if !p.in_bounds(builder.width as i32, builder.height as i32) {
                warn!("Transition {} falls outside area bounds", index);
                continue;
            }
            p.add(size.width, size.height);
            if !p.in_bounds(builder.width as i32, builder.height as i32) {
                warn!("Transition {} falls outside area bounds", index);
                continue;
            }

            debug!("Created transition to '{:?}' at {},{}", t_builder.to,
                   t_builder.from.x, t_builder.from.y);

            let transition = Transition {
                from: t_builder.from,
                to: t_builder.to,
                hover_text: t_builder.hover_text,
                size,
                image_display: image,
            };
            transitions.push(transition);
        }

        let mut triggers: Vec<Trigger> = Vec::new();
        for tbuilder in builder.triggers {
            triggers.push(Trigger {
                kind: tbuilder.kind,
                on_activate: tbuilder.on_activate,
                initially_enabled: tbuilder.initially_enabled,
            });
        }

        let mut used_triggers = HashSet::new();
        let mut encounters = Vec::new();
        for encounter_builder in builder.encounters {
            let encounter = match module.encounters.get(&encounter_builder.id) {
                None => {
                    warn!("No encounter '{}' found", &encounter_builder.id);
                    return unable_to_create_error("area", &builder.id);
                }, Some(encounter) => Rc::clone(encounter),
            };

            let mut encounter_triggers = Vec::new();
            for (index, trigger) in triggers.iter().enumerate() {
                match trigger.kind {
                    TriggerKind::OnEncounterCleared { encounter_location } |
                        TriggerKind::OnEncounterActivated { encounter_location } => {
                        if encounter_location == encounter_builder.location {
                            encounter_triggers.push(index);
                            used_triggers.insert(index);
                        }
                    },
                    _ => (),
                }
            }

            encounters.push(EncounterData {
                encounter,
                location: encounter_builder.location,
                size: encounter_builder.size,
                triggers: encounter_triggers,
            });
        }

        for (index, trigger) in triggers.iter().enumerate() {
            match trigger.kind {
                TriggerKind::OnEncounterCleared { encounter_location } |
                    TriggerKind::OnEncounterActivated { encounter_location } => {
                    if !used_triggers.contains(&index) {
                        warn!("Invalid encounter trigger at point {:?}", encounter_location);
                    }
                },
                _ => (),
            }
        }

        let visibility_tile = ResourceSet::get_sprite(&builder.visibility_tile)?;
        let explored_tile = ResourceSet::get_sprite(&builder.explored_tile)?;

        Ok(Area {
            id: builder.id,
            name: builder.name,
            width: builder.width as i32,
            height: builder.height as i32,
            layer_set: layer_set,
            path_grids: path_grids,
            actors: builder.actors,
            encounters,
            props,
            visibility_tile,
            explored_tile,
            transitions,
            triggers,
            vis_dist: builder.max_vis_distance,
            vis_dist_squared: builder.max_vis_distance * builder.max_vis_distance,
            vis_dist_up_one_squared: builder.max_vis_up_one_distance * builder.max_vis_up_one_distance,
            world_map_location: builder.world_map_location,
            on_rest: builder.on_rest,
            location_kind: builder.location_kind,
        })
    }

    pub fn coords_valid(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 { return false; }
        if x >= self.width || y >= self.height { return false; }

        true
    }

    pub fn get_path_grid(&self, size_id: &str) -> &PathFinderGrid {
        self.path_grids.get(size_id).unwrap()
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct AreaBuilder {
    pub id: String,
    pub name: String,
    pub width: usize,
    pub height: usize,
    pub visibility_tile: String,
    pub explored_tile: String,
    pub max_vis_distance: i32,
    pub max_vis_up_one_distance: i32,
    pub world_map_location: Option<String>,
    pub on_rest: OnRest,
    pub location_kind: LocationKind,
    pub generate: bool,
    pub layers: Vec<String>,
    pub entity_layer: usize,
    pub actors: Vec<ActorData>,
    pub props: Vec<PropDataBuilder>,
    pub encounters: Vec<EncounterDataBuilder>,
    pub transitions: Vec<TransitionBuilder>,
    pub triggers: Vec<TriggerBuilder>,

    #[serde(serialize_with="ser_terrain", deserialize_with="de_terrain")]
    pub terrain: Vec<Option<String>>,

    #[serde(serialize_with="ser_walls", deserialize_with="de_walls")]
    pub walls: Vec<(u8, Option<String>)>,

    #[serde(serialize_with="ser_layer_set", deserialize_with="de_layer_set")]
    pub layer_set: HashMap<String, Vec<Vec<u16>>>,

    #[serde(serialize_with="as_base64", deserialize_with="from_base64")]
    pub elevation: Vec<u8>,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
struct U8WithKinds {
    kinds: Vec<String>,
    entries: String,
}

fn entry_index<'a>(map: &mut HashMap<&'a str, u8>, index: &mut u8,
               entry: &'a Option<String>) -> Result<u8, Error> {
    Ok(match entry {
        None => 255,
        Some(ref id) => {
            let index = map.entry(id).or_insert_with(|| {
                let ret_val = *index;
                *index += 1;
                ret_val
            });

            if *index > 254 {
                return Err(Error::new(ErrorKind::InvalidInput,
                                      "Can only serialize up to 255 wall kinds"));
            }

            *index
        }
    })
}

fn serialize_u8_with_kinds<'a, S>(kinds: HashMap<&str, u8>, name: &'static str,
    vec: Vec<u8>, serializer: S) -> Result<S::Ok, S::Error> where S:Serializer {

    let mut kinds: Vec<_> = kinds.into_iter().collect();
    kinds.sort_by_key(|k| k.1);
    let kinds = kinds.into_iter().map(|k| k.0).collect::<Vec<&str>>();

    let mut data = serializer.serialize_struct(name, 2)?;
    data.serialize_field("kinds", &kinds)?;
    data.serialize_field("entries", &base64::encode(&vec))?;
    data.end()
}

fn de_terrain<'de, D>(deserializer: D) -> Result<Vec<Option<String>>, D::Error>
    where D:Deserializer<'de> {

    let input = U8WithKinds::deserialize(deserializer)?;
    use sulis_core::serde::de::Error;
    let vec_u8 = base64::decode(&input.entries).map_err(|err| Error::custom(err.to_string()))?;

    let mut out = Vec::new();
    for entry in vec_u8 {
        let index = entry as usize;
        if index== 255 {
            out.push(None);
        } else if index >= input.kinds.len() {
            return Err(Error::custom("Invalid base64 encoding in terrain index."));
        } else {
            out.push(Some(input.kinds[index].clone()));
        }
    }

    Ok(out)
}

fn ser_terrain<S>(input: &Vec<Option<String>>,
                  serializer: S) -> Result<S::Ok, S::Error> where S:Serializer {

    let mut kinds: HashMap<&str, u8> = HashMap::new();
    let mut terrain: Vec<u8> = Vec::new();

    let mut index = 0;
    for terrain_id in input.iter() {
        use sulis_core::serde::ser::Error;
        let entry_index = entry_index(&mut kinds, &mut index, terrain_id)
            .map_err(|e| Error::custom(e.to_string()))?;

        terrain.push(entry_index as u8);
    }

    serialize_u8_with_kinds(kinds, "terrain", terrain, serializer)
}

fn de_walls<'de, D>(deserializer: D) -> Result<Vec<(u8, Option<String>)>, D::Error>
    where D:Deserializer<'de> {

    let input = U8WithKinds::deserialize(deserializer)?;
    use sulis_core::serde::de::Error;
    let vec_u8 = base64::decode(&input.entries).map_err(|err| Error::custom(err.to_string()))?;

    let mut out = Vec::new();
    let mut i = 0;
    loop {
        if i + 2 > vec_u8.len() { return Err(Error::custom("Invalid base64 encoding in walls")); }

        let elev = vec_u8[i + 1];
        let index = vec_u8[i] as usize;

        if index == 255 {
            out.push((elev, None));
        } else if index >= input.kinds.len() {
            return Err(Error::custom("Invalid base64 encoding in walls index"));
        } else {
            out.push((elev, Some(input.kinds[index].clone())));
        }

        i += 2;
        if i == vec_u8.len() { break; }
    }

    Ok(out)
}

fn ser_walls<S>(input: &Vec<(u8, Option<String>)>,
                serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut kinds: HashMap<&str, u8> = HashMap::new();
    let mut walls: Vec<u8> = Vec::new();

    let mut index = 0;
    for (level, wall_id) in input.iter() {
        use sulis_core::serde::ser::Error;
        let entry_index = entry_index(&mut kinds, &mut index, wall_id)
            .map_err(|e| Error::custom(e.to_string()))?;

        walls.push(entry_index as u8);
        walls.push(*level);
    }

    serialize_u8_with_kinds(kinds, "walls", walls, serializer)
}

fn ser_layer_set<S>(input: &HashMap<String, Vec<Vec<u16>>>,
                    serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let mut map = serializer.serialize_map(Some(input.len()))?;
    for (key, vec) in input.iter() {
        let mut out: Vec<u8> = Vec::new();
        for pos in vec.iter() {
            out.push(((pos[0] >> 8) & 0xff) as u8);
            out.push((pos[0] & 0xff) as u8);
            out.push(((pos[1] >> 8) & 0xff) as u8);
            out.push((pos[1] & 0xff) as u8);
        }
        map.serialize_entry(key, &base64::encode(&out))?;
    }

    map.end()
}

fn de_layer_set<'de, D>(deserializer: D) -> Result<HashMap<String, Vec<Vec<u16>>>, D::Error>
    where D: Deserializer<'de> {

    let input: HashMap<String, String> = HashMap::deserialize(deserializer)?;

    let mut result: HashMap<String, Vec<Vec<u16>>> = HashMap::new();
    for (key, encoded) in input {
        use sulis_core::serde::de::Error;
        let vec_u8 = base64::decode(&encoded).map_err(|err| Error::custom(err.to_string()))?;

        let mut result_vec: Vec<Vec<u16>> = Vec::new();
        let mut i = 0;
        loop {
            if i + 4 > vec_u8.len() {
                return Err(Error::custom("Invalid encoded base64 string"));
            }
            let x = vec_u8[i] as u16 * 256 + vec_u8[i + 1] as u16;
            let y = vec_u8[i + 2] as u16 * 256 + vec_u8[i + 3] as u16;
            result_vec.push(vec![x, y]);

            if i + 4 == vec_u8.len() { break; }

            i += 4;
        }
        result.insert(key, result_vec);
    }

    Ok(result)
}

fn from_base64<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error> where D: Deserializer<'de> {
    use sulis_core::serde::de::Error;
    let s = String::deserialize(deserializer)?;
    base64::decode(&s).map_err(|err| Error::custom(err.to_string()))
}

fn as_base64<S>(input: &[u8], serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    serializer.serialize_str(&base64::encode(input))
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub enum LocationKind {
    Outdoors,
    Indoors,
    Underground,
}

impl LocationKind {
    pub fn iter() -> impl Iterator<Item=&'static LocationKind> {
        use crate::area::LocationKind::*;
        [ Outdoors, Indoors, Underground].into_iter()
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub enum OnRest {
    Disabled { message: String },
    FireScript { id: String, func: String },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct TriggerBuilder {
    pub kind: TriggerKind,
    pub on_activate: Vec<OnTrigger>,
    pub initially_enabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub enum ToKind {
    Area { id: String, x: i32, y: i32 },
    CurArea { x: i32, y: i32 },
    WorldMap,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct TransitionBuilder {
    pub from: Point,
    pub size: String,
    pub to: ToKind,
    pub hover_text: String,
    pub image_display: String,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct EncounterDataBuilder {
    pub id: String,
    pub location: Point,
    pub size: Size,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PropDataBuilder {
    pub id: String,
    pub location: Point,
    #[serde(default)]
    pub items: Vec<ItemListEntrySaveState>,
    pub enabled: Option<bool>,
    #[serde(default)]
    pub hover_text: Option<String>,
}

pub fn create_prop(builder: &PropDataBuilder, module: &Module) -> Result<PropData, Error> {
    let prop = match module.props.get(&builder.id) {
        None => return unable_to_create_error("prop", &builder.id),
        Some(prop) => Rc::clone(prop),
    };

    let location = builder.location;

    let enabled = builder.enabled.unwrap_or(true);

    Ok(PropData {
        prop,
        location,
        items: builder.items.clone(),
        enabled,
        hover_text: builder.hover_text.clone(),
    })
}