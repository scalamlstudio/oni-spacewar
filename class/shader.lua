-- Shader

local Shader = {}
local shadermap = {}
shadermap[1] = require("shader/s1")
local shader = nil

function Shader:load(id)
    local shader_code = shadermap[id] or shadermap[1]
    shader = love.graphics.newShader(shader_code)
end

function Shader:set(config)
    love.graphics.setShader(shader)
    for k, v in pairs(config) do
        shader:send(k, v)
    end
end

function Shader:unset()
    love.graphics.setShader()
end

return Shader