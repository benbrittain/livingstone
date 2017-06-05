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
COPY prod-passwords.json resources/passwords.json
RUN cargo build --release
RUN mkdir gpx/
EXPOSE 8080 2121
VOLUME /resources/posts/
VOLUME /gpx
CMD cargo run --release
