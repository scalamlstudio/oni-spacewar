-- Object

local Object = {}

function Object:new(world, config)
    local object = Config:newObj(config)
    object.world = world
    object.body = love.physics.newBody(Physics.world, object.x, object.y, object.bodytype)
    object.body:setMass(object.m)
    object.body:setUserData(object)
    if object.dx and object.dy and object.s then
        local distance = math.sqrt(object.dx ^ 2 + object.dy ^ 2)
        object.body:setLinearVelocity(
            object.s * object.dx / distance,
            object.s * object.dy / distance)
    end
    function object:resetPosition()        
        object.body:setPosition(0, 0)
        object.x = 0
        object.y = 0
    end
    return object
end

return Object