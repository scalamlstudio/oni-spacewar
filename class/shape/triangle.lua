-- Triangle

local Triangle = {}

function Triangle:new(world, config)
    local object = Object:new(world, config)
    object.alias = object.alias or 2
    object.shape = love.physics.newCircleShape(object.r)
    object.fixture = love.physics.newFixture(object.body, object.shape)
    object.fixture:setCategory(object.alias)
    object.fixture:setMask(object.alias)
    function object:draw()
        love.graphics.setColor(0.9, 0.9, 0.9)
        object.degree = object.degree or 0
        object.degree = object.degree + 5
        local t1x = math.cos(object.degree * 0.0174533)
        local t1y = math.sin(object.degree * 0.0174533)
        local t2x = math.cos((object.degree+120) * 0.0174533)
        local t2y = math.sin((object.degree+120) * 0.0174533)
        local t3x = math.cos((object.degree+240) * 0.0174533)
        local t3y = math.sin((object.degree+240) * 0.0174533)
        love.graphics.polygon("fill",
            object.x + t1x * object.r, object.y + t1y * object.r,
            object.x + t2x * object.r, object.y + t2y * object.r,
            object.x + t3x * object.r, object.y + t3y * object.r)
    end
    return object
end

return Triangle