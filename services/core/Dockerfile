FROM elixir:1.20.3-otp-29-slim AS build

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
ENV MIX_ENV=prod

RUN mix local.hex --force && mix local.rebar --force

COPY mix.exs mix.lock ./
RUN mix deps.get --only prod
RUN mkdir config
COPY config/config.exs config/prod.exs config/
RUN mix deps.compile

COPY priv priv
COPY lib lib
COPY config/runtime.exs config/
RUN mix release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends libstdc++6 openssl libncurses6 ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --uid 10001 core

WORKDIR /app
ENV LANG=C.UTF-8 \
    PHX_SERVER=true \
    PORT=3002

COPY --from=build --chown=core:core /app/_build/prod/rel/core ./

USER core
EXPOSE 3002

CMD ["sh", "-c", "bin/core eval 'Core.Release.migrate()' && exec bin/core start"]
