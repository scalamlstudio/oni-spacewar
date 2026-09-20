-- Hexagon

local Hexagon = {}

function Hexagon:new(world, config)
    local object = Object:new(world, config)
    object.alias = object.alias or 4
    object.shape = love.physics.newCircleShape(object.r)
    object.fixture = love.physics.newFixture(object.body, object.shape)
    object.fixture:setCategory(object.alias)
    object.fixture:setMask(object.alias)
    function object:draw()
        love.graphics.setColor(0.9, 0.0, 0.9)
        object.degree = object.degree or 0
        object.degree = object.degree + 0.5
        local t1x = math.cos(object.degree * 0.0174533)
        local t1y = math.sin(object.degree * 0.0174533)
        local t2x = math.cos((object.degree+60) * 0.0174533)
        local t2y = math.sin((object.degree+60) * 0.0174533)
        local t3x = math.cos((object.degree+120) * 0.0174533)
        local t3y = math.sin((object.degree+120) * 0.0174533)
        local t4x = math.cos((object.degree+180) * 0.0174533)
        local t4y = math.sin((object.degree+180) * 0.0174533)
        local t5x = math.cos((object.degree+240) * 0.0174533)
        local t5y = math.sin((object.degree+240) * 0.0174533)
        local t6x = math.cos((object.degree+300) * 0.0174533)
        local t6y = math.sin((object.degree+300) * 0.0174533)
        love.graphics.polygon("fill",
            object.x + t1x * object.r, object.y + t1y * object.r,
            object.x + t2x * object.r, object.y + t2y * object.r,
            object.x + t3x * object.r, object.y + t3y * object.r,
            object.x + t4x * object.r, object.y + t4y * object.r,
            object.x + t5x * object.r, object.y + t5y * object.r,
            object.x + t6x * object.r, object.y + t6y * object.r)
    end
    return object
end

return Hexagon