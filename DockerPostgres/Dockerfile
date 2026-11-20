# Keep Timescale extension ABI; update OS/PostgreSQL patch packages before promotion.
FROM timescale/timescaledb:2.32.0-pg17@sha256:6a6b7d7a346d7c6be03a310c240706f3bbc6d7adbdf3fbc07823570b34337f5e
USER root
RUN apk upgrade --no-cache && apk add --no-cache su-exec \
    && test -f /usr/local/bin/gosu \
    && rm /usr/local/bin/gosu /usr/local/bin/timescaledb-parallel-copy \
    && sed -i 's/exec gosu postgres/exec su-exec postgres/g' /usr/local/bin/docker-entrypoint.sh \
    && ! grep -q 'exec gosu postgres' /usr/local/bin/docker-entrypoint.sh
# The entrypoint only calls `gosu postgres COMMAND`; su-exec implements this form.
# parallel-copy is unused: recovery deliberately uses serial pg_restore.
