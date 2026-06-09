FROM postgres:16-alpine

LABEL maintainer="53430981+CristiCriss17G@users.noreply.github.com"
LABEL version="1.0"
LABEL title="Clone PostgreSQL database"
LABEL description="This is a Dockerfile for cloning a PostgreSQL database. For more information visit run with -h."

RUN apk update && apk --no-cache upgrade && apk add --no-cache bash && rm -rf /var/cache/apk/*

ENV PGHOST localhost
ENV PGUSER postgres
ENV DATABASE_SOURCE db1
ENV DATABASE_TARGET db2
ENV NEW_OWNER user
ENV PASSWORD ""
ENV RUN_IN_DOCKER 1
ENV KEEP_BACKUP 0

WORKDIR /usr/src/app

COPY clone_db_postgresql.sh ./

RUN chmod +x clone_db_postgresql.sh

VOLUME [ "/tmp/psql_backup" ]

ENTRYPOINT ["./clone_db_postgresql.sh"]
