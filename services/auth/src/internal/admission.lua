-- Atomic weighted quota + optional expiring concurrency reservation.
-- One Redis primary and consistent settings are required across all Auth replicas.
local capacity, rate, cost = tonumber(ARGV[1]), tonumber(ARGV[2]), tonumber(ARGV[3])
local maximum, ttl = tonumber(ARGV[4]), tonumber(ARGV[5])
local clock = redis.call('TIME')
local now = tonumber(clock[1]) * 1000 + math.floor(tonumber(clock[2]) / 1000)
local stored = redis.call('HMGET', KEYS[1], 'tokens', 'updated_ms')
local tokens = tonumber(stored[1]) or capacity
local updated = tonumber(stored[2]) or now
local effective = math.max(now, updated)
tokens = math.min(capacity, tokens + (effective - updated) * rate / 1000)
if maximum > 0 then
    redis.call('ZREMRANGEBYSCORE', KEYS[2], '-inf', now)
    if redis.call('ZCARD', KEYS[2]) >= maximum then
        local earliest = redis.call('ZRANGE', KEYS[2], 0, 0, 'WITHSCORES')
        return {0, math.max(1, math.ceil((tonumber(earliest[2]) - now) / 1000))}
    end
end
if tokens < cost then
    return {0, math.max(1, math.ceil(((cost - tokens) * 1000 / rate + effective - now) / 1000))}
end
tokens = tokens - cost
redis.call('HSET', KEYS[1], 'tokens', tokens, 'updated_ms', effective)
redis.call('PEXPIRE', KEYS[1], math.max(1, math.ceil(capacity * 1000 / rate + effective - now)))
if maximum > 0 then
    redis.call('ZADD', KEYS[2], now + ttl, ARGV[6])
    redis.call('PEXPIRE', KEYS[2], ttl)
end
return {1, 0}
