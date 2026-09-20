-- Item

local Item = {}
local itemmap = {}
itemmap[1] = {itype="map",id=1}
itemmap[2] = {itype="map",id=2}
function Item:get(world, conf)
    local newconf = itemmap[conf['id']] or itemmap[1]
    newconf.o = "item"
    for k, v in pairs(newconf) do
        conf[k] = v
    end
    local object = Hexagon:new(world, conf)
    function object:update(dt)
    end
    function object:handle(event)
    end
    return object
end

return Item