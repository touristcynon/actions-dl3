FROM ubuntu:latest

WORKDIR /home/

COPY . .

RUN bash ./setup.sh

ENV PATH="/root/.cargo/bin:/usr/local/go/bin:$PATH"
SHELL ["/usr/bin/zsh"]