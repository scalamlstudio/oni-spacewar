-- Obstacle

local Obstacle = {}
local obstaclemap = {}
obstaclemap[1] = {r=400,hp=1.427e8}
obstaclemap[2] = {r=200,hp=10}
function Obstacle:get(world, conf)
    local newconf = obstaclemap[conf['id']] or obstaclemap[1]
    newconf.o = "obst"
    newconf.t = 1.427e8
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = Square:new(world, conf)
    function object:update(dt)
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

return Obstacle