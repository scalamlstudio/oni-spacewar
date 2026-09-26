-- Enemy

local Enemy = {}
local enemymap = {}
enemymap[1] = require("enemy/e1")
enemymap[2] = require("enemy/e2")
function Enemy:get(world, conf)
    local newconf = enemymap[conf['id']] or enemymap[1]
    newconf.o = "enem"
    newconf.alias = 2
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = Triangle:new(world, conf)
    function object:update(dt)
        local char = object.world:findChar()
        local dx = char.x - object.x
        local dy = char.y - object.y
        local distance = math.sqrt(dx ^ 2 + dy ^ 2)
        dx = object.s * dx / distance
        dy = object.s * dy / distance
        object.body:setLinearVelocity(dx, dy)
        object.x = object.body:getX()
        object.y = object.body:getY()
        if object.etype == "range" and math.random() > 0.98 then
            local conf = {id=2, alias=2, x=object.x, y=object.y, dx=dx, dy=dy}
            local newobj = Projectile:get(object.world, conf)
            object.world:addObj(newobj)
        end
    end
    function object:handle(event)
        if event.etype == "hit" then
        elseif event.etype == "tmat" then
            if event.time == 0 then
                object[event.tar] = object[event.tar] * event.mul
                object[event.tar] = object[event.tar] + event.add
            end
        end
    end
    return object
end

return Enemy