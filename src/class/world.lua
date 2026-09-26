-- World

local World = {}
local worldmap = {}
worldmap[1] = require("world/w1")
worldmap[2] = require("world/w2")
function World:new(conf)
    local world = conf or {}
    world.id = world.id or 1
    world.w = love.graphics.getWidth()
    world.h = love.graphics.getHeight()
    world.objs = world.objs or {}
    world.char = world.char or nil
    world.port = 0
    function world:update(dt, paras)
        world.w = love.graphics.getWidth()
        world.h = love.graphics.getHeight()

        local objsize = #world.objs
        for i = 1, objsize do
            world.objs[i]:update(dt)
            if world.objs[i].t <= 0 or world.objs[i].hp <= 0 then
                world.objs[i].a = false
                world.objs[i].body:destroy()
            end
        end

        local newobjs = {}
        for i = 1, #world.objs do
            if world.objs[i].a then
                table.insert(newobjs, world.objs[i])
            end
        end
        world.objs = newobjs
    end
    function world:addObj(obj)
        table.insert(world.objs, obj)
    end
    function world:clear()
        for i = 1, #world.objs do
            world.objs[i].t = 0
        end
    end
    function world:findChar()
        if world.char == nil then
            for i = 1, #world.objs do
                if world.objs[i].o == "play" then
                    world.char = world.objs[i]
                end
            end
        end
        return world.char
    end
    function world:initLevel()
        -- items
        local nx = love.math.random(-200, 200)
        local ny = love.math.random(-200, 200)
        local conf = {id=1, x=nx, y=ny}
        local obj = Portal:get(world, conf)
        table.insert(world.objs, obj)
        -- obstacles
        for k, v in pairs(worldmap[world.id].obstacle) do
            for i = 1, v do
                local nx = love.math.random(-1000, 1000)
                local ny = love.math.random(-1000, 1000)
                if nx^2 + ny^2 > 100000 then
                    local conf = {o="obst", x=nx, y=ny, id=k}
                    local obj = Obstacle:get(world, conf)
                    table.insert(world.objs, obj)
                end
            end
        end
        -- enemies
        for k, v in pairs(worldmap[world.id].enemy) do
            for i = 1, v do
                local nx = love.math.random(-1000, 1000)
                local ny = love.math.random(-1000, 1000)
                if nx^2 + ny^2 > 1000 then
                    local conf = {x=nx, y=ny, id=k}
                    local obj = Enemy:get(world, conf)
                    table.insert(world.objs, obj)
                end
            end
        end
    end
    function world:draw()
        for i = 1, #world.objs do
            world.objs[i]:draw()
        end
    end
    return world
end

return World