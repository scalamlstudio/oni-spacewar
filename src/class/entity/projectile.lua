-- Projectile

local Projectile = {}
local projmap = {}
projmap[1] = require("projectile/p1")
projmap[2] = require("projectile/p2")
function Projectile:get(world, conf)
    local newconf = projmap[conf['id']] or projmap[1]
    newconf.o = "proj"
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = {}
    if conf.ptype == "triangle" then
        object = Triangle:new(world, conf)
    else
        object = Circle:new(world, conf)
    end
    function object:update(dt)
        object.x = object.body:getX()
        object.y = object.body:getY()
        object.t = object.t - dt
    end
    function object:handle(event)
        if event.etype == "hit" then
            object.hp = object.hp - 1
            for ei, e in pairs(object.effect) do
                effect = {}
                for k, v in pairs(e) do
                    effect[k] = v
                end
                event.object:handle(effect)
            end
        end
    end
    return object
end

return Projectile