function on_activate(parent, ability)
  if parent:stats().is_move_disabled then return end

  local targets = parent:targets():hostile():visible()
  
  local targeter = parent:create_targeter(ability)
  targeter:set_selection_radius(ability:range())
  targeter:set_free_select(ability:range())
  targeter:impass_blocks_affected_points(true)
  targeter:set_shape_line_segment(parent:size_str(), parent:x(), parent:y())
  targeter:add_all_effectable(targets)
  targeter:activate()
end

function on_target_select(parent, ability, targets)
  local pos = targets:selected_point()
  
  local cb = ability:create_callback(parent)
  cb:add_targets(targets)
  cb:set_on_anim_complete_fn("move_parent")
  
  local speed = 300 * game:anim_base_time()
  local dist = parent:dist_to_point(pos)
  local duration = dist / speed
  
  local anim = parent:create_subpos_anim(duration)

  local delta_x = pos.x - parent:x()
  local delta_y = pos.y - parent:y()
  
  anim:set_position(anim:param(0.0, delta_x / duration), anim:param(0.0, delta_y / duration))
  anim:set_completion_callback(cb)
  
  local targets = targets:to_table()
  for i = 1, #targets do 
    local dist = parent:dist_to_entity(targets[i])
    local duration = dist / speed
    
    local cb = ability:create_callback(parent)
	cb:add_target(targets[i])
	cb:set_on_anim_update_fn("attack_target")
    anim:add_callback(cb, duration)
  end
  
  anim:activate()
  ability:activate(parent)
end

function attack_target(parent, ability, targets)
  local target = targets:first()

  if not target:is_valid() then return end
  
  local hit = parent:weapon_attack(target)
  
  local base_dist = math.floor(10 + 2 * parent:width() - 2 * target:width())
  local direction = -1
  
  for i = 1, 5 do
    local point = pick_random_point(target:x(), target:y())
    local dist = push_target(base_dist, target, hit, point, direction)
	if dist > 0 then break end
  end
end

function move_parent(parent, ability, targets)
  local dest = targets:selected_point()
  parent:teleport_to(dest)
end

function pick_random_point(x, y)
  return {x = x + math.random(-5, 5), y = y + math.random(-5, 5)}
end

--INCLUDE push_target