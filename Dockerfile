FROM alpine:edge

RUN apk update
RUN apk add git \
            make \
            file \
            openssl \
            openssl-dev \
            rust \
            cargo

RUN git clone https://github.com/cavedweller/livingstone.git
WORKDIR livingstone
RUN cargo build --release
