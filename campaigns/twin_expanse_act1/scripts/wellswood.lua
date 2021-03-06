function goblin_raids_start(parent)
  game:set_quest_entry_state("leader_of_beasts", "start", "Visible")
end

function goblin_raids_leads(parent)
  game:set_quest_entry_state("leader_of_beasts", "leads", "Visible")
end

function cragnik_join(parent)
  game:add_party_member("npc_cragnik")
end

function jhilsara_join(parent)
  game:add_party_member("npc_jhilsara")
  game:set_quest_entry_state("leader_of_beasts", "jhilsara", "Visible")
end

function enter_square(parent)
  game:cancel_blocking_anims()
  game:scroll_view(98, 47)
  game:start_conversation("wellswood_enter_square", parent)
end

function enter_docks(parent)
  game:cancel_blocking_anims()
  game:scroll_view(61, 107)
  game:start_conversation("wellswood_enter_docks", parent)
end

function view_docks_thug(parent)
  game:set_quest_entry_state("the_thug", "docks_view", "Visible")
end

function docks_thugs_attack(parent)
  local worker = game:entity_with_id("dock_foreman")
  worker:set_flag("player_fought")

  local thug01 = game:entity_with_id("thug01")
  local thug02 = game:entity_with_id("thug02")
  local thug03 = game:entity_with_id("thug03")
  
  thug01:set_faction("Hostile")
  thug02:set_faction("Hostile")
  thug03:set_faction("Hostile")
end

function docks_thugs_leave_early(parent)
  game:add_party_xp(50)
  local thug01 = game:entity_with_id("thug01")
  game:say_line("That will teach you!  Have the money next time or else!")
  
  local worker = game:entity_with_id("dock_foreman")
  worker:set_flag("player_didnt_help")
  worker:take_damage(thug01, 6, 6, "Raw")
  game:run_script_delayed("wellswood", "docks_thugs_leave", 2.0)
end

function docks_thugs_leave_helped(parent)
  local worker = game:entity_with_id("dock_foreman")
  worker:set_flag("player_helped")
  
  docks_thugs_leave(parent)
end

function docks_thugs_leave(parent)
  local thug01 = game:entity_with_id("thug01")
  local thug02 = game:entity_with_id("thug02")
  local thug03 = game:entity_with_id("thug03")
  
  if not thug01:move_towards_point(75, 77) then
    game:log("thug01 unable to move")
  end
  
  if not thug02:move_towards_point(77, 77) then
    game:log("thug02 unable to move")
  end
  
  if not thug03:move_towards_point(79, 77) then
    game:log("thug02 unable to move")
  end
  
  game:run_script_delayed("wellswood", "docks_thugs_leave_finish", 2.0)
end

function docks_thugs_leave_finish(parent)
  local target = game:entity_with_id("thug01")
  target:remove()
  
  local target = game:entity_with_id("thug02")
  target:remove()
  
  target = game:entity_with_id("thug03")
  target:remove()
end

function docks_thugs_cleared(parent)
  local worker = game:entity_with_id("dock_foreman")
  game:say_line("Alright, back to work!", worker)
end

function docks_foreman_info(parent)
  game:add_party_xp(50)
  local worker = game:entity_with_id("dock_foreman")
  worker:set_flag("got_info")
  game:set_quest_entry_state("the_thug", "docks_info", "Visible")
  
  game:set_world_map_location_visible("thugs_hideout", true)
  game:set_world_map_location_enabled("thugs_hideout", true)
end

function smith_info(parent)
  game:add_party_xp(50)
  game:set_quest_entry_state("the_thug", "docks_info", "Visible")
  
  game:set_world_map_location_visible("thugs_hideout", true)
  game:set_world_map_location_enabled("thugs_hideout", true)
  
  local smith = game:entity_with_id("smith01")
  local coins = math.floor(smith:get_num_flag("coins_to_take"))
  if coins > 0 then
    game:add_party_coins(-coins)
  end
end

function thugs_reward(parent)
  game:add_party_xp(100)
  game:add_party_coins(3000)
  game:set_quest_state("the_thug", "Complete")
  game:player():clear_flag("gethruk_cleared")
end

function priest_rest(parent)
  game:run_script_delayed("campaign", "fire_rest", 0.0)
end

function get_rose_lake_pass(parent)
  game:add_party_xp(200)
  game:add_party_coins(5000)
  
  game:set_quest_entry_state("entering_rose_lake", "got_pass", "Visible")
  game:set_quest_state("entering_rose_lake", "Complete")
  
  game:set_quest_entry_state("leader_of_beasts", "complete", "Visible")
  game:set_quest_state("leader_of_beasts", "Complete")
  
  game:add_party_item("rose_lake_pass")
  
  game:player():set_flag("leader_of_beasts_reported")

  game:set_quest_entry_state("seeing_the_council", "start", "Visible")
end
