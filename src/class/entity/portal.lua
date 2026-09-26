-- Portal

local Portal = {}
local portalmap = {}
portalmap[1] = {r=50, hp=1.427e8, t=300}
function Portal:get(world, conf)
    local newconf = portalmap[conf['id']] or portalmap[1]
    newconf.o = "port"
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = Hexagon:new(world, conf)
    function object:update(dt)
    end
    function object:handle(event)
        if event.etype == "hit" then
            if event.object.o == "play" then
                world.port = math.random(1, 2)
            end
        end
    end
    return object
end

return Portal