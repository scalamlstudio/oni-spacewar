-- Config

local Config = {}

function Config:newObj(config)
    local config = config
    config.a = config.a or true -- alive
    config.x = config.x or 0 -- position x
    config.y = config.y or 0 -- position y
    config.t = config.t or 1 -- life time
    config.s = config.s or 1000 -- speed
    config.maxs = config.s -- max speed
    config.r = config.r or 1 -- radius
    config.m = config.m or 1 -- mass
    config.hp = config.hp or 1 -- health point
    config.maxhp = config.hp -- max health point
    config.effs = {}
    config.alias = config.alias or 3 -- default neutral
    config.o = config.o or "obst" -- object type
    if config.o == "obst" or config.o == "port" then
        config.bodytype = "static"
    else
        config.bodytype = "dynamic"
    end
    return config
end

return Config