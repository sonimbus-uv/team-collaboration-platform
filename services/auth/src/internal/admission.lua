-- Atomic weighted quota + optional expiring concurrency reservation.
-- One Redis primary and consistent settings are required across all Auth replicas.
local capacity = tonumber(ARGV[1])
local refill_per_second = tonumber(ARGV[2])
local pair_cost = tonumber(ARGV[3])
local concurrent_limit = tonumber(ARGV[4])
local lease_ttl_ms = tonumber(ARGV[5])

local redis_time = redis.call("TIME")
local now_ms = tonumber(redis_time[1]) * 1000 + math.floor(tonumber(redis_time[2]) / 1000)

local stored_bucket = redis.call("HMGET", KEYS[1], "tokens", "updated_ms")
local available_tokens = tonumber(stored_bucket[1]) or capacity
local updated_ms = tonumber(stored_bucket[2]) or now_ms
local effective_now_ms = math.max(now_ms, updated_ms)

local refilled_tokens = (effective_now_ms - updated_ms) * refill_per_second / 1000
available_tokens = math.min(capacity, available_tokens + refilled_tokens)

if concurrent_limit > 0 then
    redis.call("ZREMRANGEBYSCORE", KEYS[2], "-inf", now_ms)
    if redis.call("ZCARD", KEYS[2]) >= concurrent_limit then
        local oldest_lease = redis.call("ZRANGE", KEYS[2], 0, 0, "WITHSCORES")
        local retry_after_secs = math.max(1, math.ceil((tonumber(oldest_lease[2]) - now_ms) / 1000))
        return { 0, retry_after_secs }
    end
end

if available_tokens < pair_cost then
    local retry_after_secs = math.max(
        1,
        math.ceil(((pair_cost - available_tokens) * 1000 / refill_per_second + effective_now_ms - now_ms) / 1000)
    )
    return { 0, retry_after_secs }
end

available_tokens = available_tokens - pair_cost

redis.call("HSET", KEYS[1], "tokens", available_tokens, "updated_ms", effective_now_ms)
local quota_ttl_ms = math.max(
    1,
    math.ceil(capacity * 1000 / refill_per_second + effective_now_ms - now_ms)
)
redis.call("PEXPIRE", KEYS[1], quota_ttl_ms)

if concurrent_limit > 0 then
    redis.call("ZADD", KEYS[2], now_ms + lease_ttl_ms, ARGV[6])
    redis.call("PEXPIRE", KEYS[2], lease_ttl_ms)
end

return { 1, 0 }
