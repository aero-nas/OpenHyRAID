FROM --platform=linux/amd64 alpine:latest

WORKDIR /app

RUN mkdir loopdevimg loopdev scripts src src/crates 

COPY ./crates /app/src/crates
COPY ./Cargo* /app/src/

COPY --chmod=0755 ./docker-ci/scripts/*.sh /app/scripts

RUN apk upgrade --no-cache

RUN apk add --no-cache \
    bash \
    mdadm \
    lvm2 \
    rustup \
    util-linux-misc \
    sfdisk \
    build-base

RUN rustup-init -y
RUN echo 'export PATH="/root/.cargo/bin:$PATH"' >> /root/.bashrc

CMD [ "/app/scripts/main.sh", "x86_64-musl-linux" ]