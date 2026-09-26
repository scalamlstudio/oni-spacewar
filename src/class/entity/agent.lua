-- Agent

local Agent = {}
local agentmap = {}
agentmap[1] = {t=1.427e8}
function Agent:get(world, conf)
    local newconf = agentmap[conf['id']] or agentmap[1]
    newconf.o = "play"
    newconf.alias = 1
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = Circle:new(world, conf)
    function object:update(dt)
        object.s = object.maxs
        local neweffs = {}
        for ei, e in pairs(object.effs) do
            if e.etype == "tmat" then
                local old_time = e.time
                e.time = e.time - dt
                if e.time > 0 then
                    object[e.tar] = object[e.tar] * e.mul
                    object[e.tar] = object[e.tar] + e.add
                    table.insert(neweffs, e)
                end
            end
        end
        object.effs = neweffs
        -- object.world.log:add(tostring(#neweffs))
        local dx = love.mouse.getX() - love.graphics.getWidth() / 2
        local dy = love.mouse.getY() - love.graphics.getHeight() / 2
        local distance = math.sqrt(dx ^ 2 + dy ^ 2)
        if love.mouse.isDown(1) then
            dx = object.s * dx / distance
            dy = object.s * dy / distance
            object.body:setLinearVelocity(dx, dy)
        else
            object.body:setLinearVelocity(0, 0)
        end
        object.x = object.body:getX()
        object.y = object.body:getY()
        if object.world.control:onSkill(1) then
            local conf = {id=1, alias=1, x=object.x, y=object.y, dx=dx, dy=dy}
            local newobj = Projectile:get(object.world, conf)
            object.world:addObj(newobj)
        end
        object.t = 1 -- NOT CLEARED BY WORLD
    end
    function object:handle(event)
        if event.etype == "hit" then
        elseif event.etype == "tmat" then
            if event.time == 0 then
                object[event.tar] = object[event.tar] * event.mul
                object[event.tar] = object[event.tar] + event.add
            else
                table.insert(object.effs, event)
            end
        end
    end
    return object
end

return Agent