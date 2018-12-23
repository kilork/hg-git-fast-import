FROM rust

RUN apt-get update && apt-get install -y --no-install-recommends \
    git \
    python-setuptools \
    python-pip \
    python2.7-dev \
    openssh-client \
    procps \
    && rm -rf /var/lib/apt/lists/*

RUN pip install 'Mercurial>=4.8.0,<4.9.0'

RUN cd $HOME && \
    git clone https://github.com/kilork/hg-git-fast-import.git && \
    cd hg-git-fast-import && \
    cargo install --path .
