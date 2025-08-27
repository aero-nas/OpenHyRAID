FROM --platform=linux/amd64 ubuntu:latest

WORKDIR /app

RUN mkdir loopdevimg loopdev scripts src src/crates 

COPY ./crates /app/src/crates
COPY ./Cargo* /app/src/

COPY --chmod=0755 ./docker-ci/scripts/*.sh /app/scripts

RUN apt update -y && apt upgrade -y

RUN apt install \
    mdadm \
    lvm2 \
    rustup \
    fdisk \
    -y

RUN rustup default stable

CMD [ "/app/scripts/main.sh", "x86_64-gnu-linux" ]