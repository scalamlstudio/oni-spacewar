-- Control

local Control = {}

function Control:keyboard()
    local control = {}
    local keys = {}
    -- hotkey items -> 1, 2, 3, 4
    local item = {'1', '2', '3', '4'}
    -- hotkey skills -> 1, 2, 3, 4
    local skill = {'q', 'w', 'e', 'r'}
    -- map, item, operate, player
    local system = {'m', 'i', 'o', 'p'}
    function control:onItem(i)
        if i > 0 and i <= #item and keys[item[i]] then
            return true
        end
        return false
    end
    function control:onSkill(i)
        if i > 0 and i <= #skill and keys[skill[i]] then
            return true
        end
        return false
    end
    function control:onSystem(i)
        if i > 0 and i <= #system and keys[system[i]] then
            return true
        end
        return false
    end
    function control:update(dt)
        for key, keytime in pairs(keys) do
            if not love.keyboard.isDown(key) then
                keys[key] = nil
            end
        end
    end
    function love.keypressed(key, scancode, isrepeat)
        keys[key] = love.timer.getTime()
        if key == "escape" then love.event.quit() end
    end
    return control
end

return Control