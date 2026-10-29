local capacity = tonumber(ARGV[1])
local refill_ms = tonumber(ARGV[2])

if not capacity or capacity < 1 or not refill_ms or refill_ms < 1 then
    return redis.error_reply("ERR invalid bucket policy")
end

local clock = redis.call("TIME")
local now_ms = tonumber(clock[1]) * 1000 + math.floor(tonumber(clock[2]) / 1000)

local stored = redis.call("HMGET", KEYS[1], "tokens", "updated_ms")
local tokens = tonumber(stored[1])
local updated_ms = tonumber(stored[2])

if not tokens or not updated_ms then
    tokens = capacity
    updated_ms = now_ms
end

local effective_now = math.max(now_ms, updated_ms)
local elapsed_ms = effective_now - updated_ms
tokens = math.min(capacity, tokens + elapsed_ms * capacity / refill_ms)

local allowed = 0
local retry_ms = 0

if tokens >= 1 then
    tokens = tokens - 1
    allowed = 1
else
    retry_ms = math.ceil((1 - tokens) * refill_ms / capacity)
end

local ttl_ms = math.max(
    1,
    math.ceil((capacity - tokens) * refill_ms / capacity)
        + effective_now - now_ms
)

redis.call("HSET", KEYS[1], "tokens", tokens, "updated_ms", effective_now)
redis.call("PEXPIRE", KEYS[1], ttl_ms)

return {allowed, retry_ms}