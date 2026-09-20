-- Char

local Char = {}

function Char:new(name)
    local char = {}
    char.name = name
    char.hp = 100
    char.str = 0
    char.dex = 0
    char.int = 0
    char.s = 400
    char.r = 10
    char.m = 5
    char.skills = {}
    char.equips = {}
    char.items = {}
    return char
end

return Char
