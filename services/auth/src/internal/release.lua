local lease_expiration = redis.call("ZSCORE", KEYS[1], ARGV[1])
local redis_time = redis.call("TIME")
local now_ms = tonumber(redis_time[1]) * 1000 + math.floor(tonumber(redis_time[2]) / 1000)

redis.call("ZREM", KEYS[1], ARGV[1])

if lease_expiration and tonumber(lease_expiration) > now_ms then
    return { 1 }
else
    return { 0 }
end
