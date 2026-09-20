-- Square

local Square = {}

function Square:new(world, config)
    local object = Object:new(world, config)
    object.alias = object.alias or 0
    object.shape = love.physics.newRectangleShape(object.r, object.r)
    object.fixture = love.physics.newFixture(object.body, object.shape)
    object.fixture:setCategory(object.alias)
    object.fixture:setMask(object.alias)
    function object:draw()
        if object.hp > 1e7 then
            love.graphics.setColor(1.0, 0.7, 0.2)
        else
            love.graphics.setColor(0.9,
                0.9 * object.hp / object.maxhp,
                0.9 * object.hp / object.maxhp)
        end
        love.graphics.rectangle("fill",
            object.x - object.r / 2, object.y - object.r / 2,
            object.r, object.r)
    end
    return object
end

return Square