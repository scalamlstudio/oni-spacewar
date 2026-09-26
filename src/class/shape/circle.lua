-- Circle

local Circle = {}

function Circle:new(world, config)
    local object = Object:new(world, config)
    object.alias = object.alias or 1
    object.shape = love.physics.newCircleShape(object.r)
    object.fixture = love.physics.newFixture(object.body, object.shape)
    object.fixture:setCategory(object.alias)
    object.fixture:setMask(object.alias)
    function object:draw()
        love.graphics.setColor(0.9, 0.9, 0.9)
        love.graphics.circle("fill",
            object.body:getX(), object.body:getY(),
            object.shape:getRadius(), 20)
    end
    return object
end

return Circle