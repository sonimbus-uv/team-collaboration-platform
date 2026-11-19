local capacity = tonumber(ARGV[1])
local refill_period_ms = tonumber(ARGV[2])

if not capacity or capacity < 1 or not refill_period_ms or refill_period_ms < 1 then
    return redis.error_reply("ERR invalid bucket policy")
end

local redis_time = redis.call("TIME")
local now_ms = tonumber(redis_time[1]) * 1000 + math.floor(tonumber(redis_time[2]) / 1000)

local stored_bucket = redis.call("HMGET", KEYS[1], "tokens", "updated_ms")
local available_tokens = tonumber(stored_bucket[1])
local last_updated_ms = tonumber(stored_bucket[2])

if not available_tokens or not last_updated_ms then
    available_tokens = capacity
    last_updated_ms = now_ms
end

local effective_now_ms = math.max(now_ms, last_updated_ms)
local elapsed_ms = effective_now_ms - last_updated_ms

available_tokens = math.min(
    capacity,
    available_tokens + elapsed_ms * capacity / refill_period_ms
)

local request_allowed = 0
local retry_after_ms = 0

if available_tokens >= 1 then
    available_tokens = available_tokens - 1
    request_allowed = 1
else
    retry_after_ms = math.ceil((1 - available_tokens) * refill_period_ms / capacity)
end

local bucket_ttl_ms = math.max(
    1,
    math.ceil((capacity - available_tokens) * refill_period_ms / capacity)
        + effective_now_ms - now_ms
)

redis.call("HSET", KEYS[1], "tokens", available_tokens, "updated_ms", effective_now_ms)
redis.call("PEXPIRE", KEYS[1], bucket_ttl_ms)

return { request_allowed, retry_after_ms }